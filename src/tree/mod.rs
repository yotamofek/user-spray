mod walk;

use syn::{
    token::{Brace, PathSep},
    Ident, UseGroup, UseName, UsePath, UseTree,
};

use self::walk::walk_use_tree;

#[derive(Debug, Clone, PartialEq, Eq)]
pub(super) struct Node {
    ident: Ident,
    children: Vec<Node>,
}

impl Node {
    fn new(ident: Ident, children: impl IntoIterator<Item = Node>) -> Self {
        Self {
            ident,
            children: children.into_iter().collect(),
        }
    }

    fn leaf(ident: Ident) -> Self {
        Self::new(ident, [])
    }
}

#[derive(Default)]
struct Visitor {
    current_path: Vec<Ident>,
    root_node: Option<Node>,
}

impl walk::Visitor for Visitor {
    fn enter_path(&mut self, ident: Ident) {
        self.current_path.push(ident);
    }

    fn leave_path(&mut self) {
        self.current_path.pop().unwrap();
    }

    fn visit_name(&mut self, ident: Ident) {
        let mut path_segments = self.current_path.iter();
        let mut node = match (&mut self.root_node, path_segments.next()) {
            // handle tree with just one leaf, e.g. `use std;`
            (None, None) => {
                self.root_node = Some(Node::leaf(ident.clone()));
                return;
            }
            (Some(root), Some(first_segment)) => {
                assert_eq!(
                    root.ident, *first_segment,
                    "trying to visit tree with different root node"
                );
                root
            }
            (None, Some(first_segment)) => self.root_node.insert(Node::leaf(first_segment.clone())),

            (Some(_), None) => unreachable!(),
        };

        for path in path_segments {
            node = if let Some((existing_node, _)) = node
                .children
                .iter()
                .enumerate()
                .find(|(_, node)| node.ident == *path)
            {
                &mut node.children[existing_node]
            } else {
                node.children.push(Node::leaf(path.clone()));
                node.children.last_mut().unwrap()
            }
        }

        node.children.push(Node::leaf(ident.clone()));
    }
}

impl FromIterator<UseTree> for Node {
    fn from_iter<T: IntoIterator<Item = UseTree>>(iter: T) -> Self {
        let mut visitor = Visitor::default();
        for tree in iter {
            walk_use_tree(tree, &mut visitor);
        }
        visitor.root_node.unwrap()
    }
}

impl From<UseTree> for Node {
    fn from(value: UseTree) -> Self {
        Self::from_iter([value])
    }
}

impl From<Node> for UseTree {
    fn from(Node { ident, children }: Node) -> Self {
        if children.is_empty() {
            Self::Name(UseName { ident })
        } else {
            Self::Path(UsePath {
                ident,
                colon2_token: PathSep::default(),
                tree: Box::new(UseTree::Group(UseGroup {
                    brace_token: Brace::default(),
                    items: children.into_iter().map(UseTree::from).collect(),
                })),
            })
        }
    }
}

#[cfg(test)]
mod tests {
    use proc_macro2::Span;
    use syn::parse_quote;

    use super::*;

    macro_rules! parse_node {
        {$($tt:tt)*} => {{
            let tree: UseTree = parse_quote! {
                $($tt)*
            };
            Node::from(tree)
        }}
    }

    fn ident(ident: &str) -> Ident {
        Ident::new(ident, Span::call_site())
    }

    macro_rules! n {
        ($name:path) => {
            Node::leaf(ident(stringify!($name)))
        };
        ($name:path, [$($node:expr),+]) => {
            Node::new(ident(stringify!($name)), [$($node),+])
        };
    }

    #[test]
    fn test_tree_to_node() {
        assert_eq!(parse_node!(std::a), n!(std, [n!(a)]));

        assert_eq!(
            parse_node!(std::{a::b, a::c}),
            n!(std, [n!(a, [n!(b), n!(c)])])
        );

        assert_eq!(
            Node::from_iter([parse_quote!(std::{a, b::c}), parse_quote!(std::{b::{d, e}})]),
            n!(std, [n!(a), n!(b, [n!(c), n!(d), n!(e)])])
        );
    }

    #[ignore] // TODO!
    #[test]
    fn test_tree_to_node_with_self() {
        assert_eq!(
            parse_node!(std::{a, a::b, a::c}),
            n!(std, [n!(a, [n!(self), n!(b), n!(c)])])
        );
    }

    #[test]
    fn test_node_to_tree() {
        assert_eq!(
            UseTree::from(n!(std, [n!(a, [n!(b), n!(c)])])),
            parse_quote!(std::{a::{b, c}})
        )
    }
}
