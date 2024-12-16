mod walk;

use std::fmt::{self, Debug};

use fn_formats::DebugFmt;
use syn::{token::Brace, Ident, Token, UseGlob, UseGroup, UseName, UsePath, UseRename, UseTree};

use self::walk::walk_use_tree;
use crate::{display::DebugAdapter, map::Name};

#[derive(Clone, PartialEq, Eq)]
pub(super) enum Node {
    Ident { ident: Ident, children: Vec<Node> },
    Glob,
    Rename { ident: Ident, rename: Ident },
}

impl Debug for Node {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Ident { ident, children } => {
                let mut tuple = f.debug_tuple("Ident");
                tuple.field(&DebugAdapter(ident));
                if !children.is_empty() {
                    tuple.field(children);
                    tuple.finish()
                } else {
                    tuple.finish_non_exhaustive()
                }
            }
            Self::Glob => write!(f, "Glob"),
            Self::Rename { ident, rename } => f
                .debug_tuple("Rename")
                .field(&DebugFmt(|f| write!(f, "{ident} as {rename}")))
                .finish(),
        }
    }
}

impl Node {
    fn ident(ident: Ident) -> Self {
        Self::Ident {
            ident,
            children: Vec::new(),
        }
    }

    fn glob() -> Self {
        Self::Glob
    }

    fn rename(ident: Ident, rename: Ident) -> Self {
        Self::Rename { ident, rename }
    }
}

impl From<Name> for Node {
    fn from(value: Name) -> Self {
        match value {
            Name::Ident(ident) => Self::ident(ident),
            Name::Glob => Self::glob(),
            Name::Rename { ident, rename } => Self::rename(ident, rename),
        }
    }
}

#[derive(Default)]
struct Visitor {
    current_path: Vec<Ident>,
    root_node: Option<Node>,
}

impl Debug for Visitor {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("Visitor")
            .field(
                "current_path",
                &DebugFmt(|f| {
                    f.debug_list()
                        .entries(self.current_path.iter().map(DebugAdapter))
                        .finish()
                }),
            )
            .field("root_node", &self.root_node)
            .finish()
    }
}

impl walk::Visitor for Visitor {
    fn enter_path(&mut self, ident: Ident) {
        self.current_path.push(ident);
    }

    fn leave_path(&mut self) {
        self.current_path.pop().unwrap();
    }

    fn visit_name(&mut self, name: Name) {
        let mut path_segments = self.current_path.iter();
        let mut node = match (&mut self.root_node, path_segments.next()) {
            // handle tree with just one leaf, e.g. `use std;`
            (None, None) => {
                self.root_node = Some(Node::from(name));
                return;
            }
            (Some(root), Some(first_segment)) => {
                assert!(
                    matches!(root, Node::Ident { ident,.. } if *ident == *first_segment),
                    "trying to visit tree with different root node"
                );
                root
            }
            (None, Some(first_segment)) => {
                self.root_node.insert(Node::ident(first_segment.clone()))
            }

            (Some(_), None) => unreachable!(),
        };

        for path in path_segments {
            let Node::Ident { children, .. } = node else {
                unreachable!()
            };

            node = if let Some((existing_node, _)) = children
                .iter()
                .enumerate()
                .find(|(_, node)| matches!(node, Node::Ident { ident, ..} if *ident == *path))
            {
                &mut children[existing_node]
            } else {
                children.push(Node::ident(path.clone()));
                children.last_mut().unwrap()
            };
        }

        let Node::Ident { children, .. } = node else {
            unreachable!();
        };

        children.push(Node::from(name));
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
    fn from(node: Node) -> Self {
        match node {
            Node::Ident { ident, children } => {
                if children.is_empty() {
                    Self::Name(UseName { ident })
                } else {
                    Self::Path(UsePath {
                        ident,
                        colon2_token: <Token![::]>::default(),
                        tree: Box::new(UseTree::Group(UseGroup {
                            brace_token: Brace::default(),
                            items: children.into_iter().map(UseTree::from).collect(),
                        })),
                    })
                }
            }
            Node::Glob => Self::Glob(UseGlob {
                star_token: <Token![*]>::default(),
            }),
            Node::Rename { ident, rename } => Self::Rename(UseRename {
                ident,
                as_token: <Token![as]>::default(),
                rename,
            }),
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
            Node::ident(ident(stringify!($name)))
        };
        ($name:path, [$($node:expr),+]) => {
            Node::Ident {
                ident: ident(stringify!($name)),
                children: vec![$($node),+],
            }
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
