mod walk;

use std::{
    borrow::Borrow,
    cell::{Cell, Ref, RefCell, RefMut},
    fmt::{self, Debug},
    io::{stderr, Write},
    mem::take,
    rc::Rc,
};

use fn_formats::DebugFmt;
use syn::{token::Brace, Ident, Token, UseGlob, UseGroup, UseName, UsePath, UseRename, UseTree};

use self::walk::walk_use_tree;
use crate::{display::DebugAdapter, map::Name};

#[derive(Clone, PartialEq, Eq)]
pub(super) enum Node {
    Parent {
        ident: Ident,
        child: Rc<RefCell<GroupOrNode>>,
    },
    Leaf(Name),
}

impl Debug for Node {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Parent { ident, child } => f
                .debug_struct("Parent")
                .field("ident", &DebugAdapter(ident))
                .field("child", &RefCell::borrow(child))
                .finish(),
            Self::Leaf(name) => f.debug_tuple("Leaf").field(name).finish(),
        }
    }
}

#[derive(Clone, PartialEq, Eq)]
pub(super) enum GroupOrNode {
    Group(Group),
    Node(Node),
}

impl Debug for GroupOrNode {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Group(group) => group.fmt(f),
            Self::Node(node) => node.fmt(f),
        }
    }
}

#[derive(Clone, Default, PartialEq, Eq)]
pub(super) struct Group {
    children: Rc<RefCell<Vec<Node>>>,
}

impl Debug for Group {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_tuple("Group")
            .field(&DebugFmt(|f| {
                f.debug_list().entries(self.children().iter()).finish()
            }))
            .finish()
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub(super) struct Tree {
    root: GroupOrNode,
}

impl Default for Tree {
    fn default() -> Self {
        Self {
            root: GroupOrNode::Group(Group::default()),
        }
    }
}

impl From<Name> for Node {
    fn from(value: Name) -> Self {
        Self::Leaf(value)
    }
}

#[derive(Default)]
struct Visitor {
    current_path: Vec<Ident>,
    tree: Tree,
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
            .field("tree", &self.tree)
            .finish()
    }
}

impl Group {
    fn new_with_self() -> Self {
        Self {
            children: Rc::new(RefCell::new(vec![Node::self_leaf()])),
        }
    }

    fn find_child_by_ident(&self, ident: &Ident) -> Option<Node> {
        self.children()
            .iter()
            .find(|child| child.parent_ident() == Some(ident))
            .cloned()
    }

    fn children(&self) -> Ref<Vec<Node>> {
        RefCell::borrow(&self.children)
    }

    fn children_mut(&self) -> RefMut<Vec<Node>> {
        self.children.borrow_mut()
    }

    fn push_child(&self, child: Node) {
        self.children_mut().push(child)
    }
}

impl Node {
    fn self_leaf() -> Self {
        Node::Leaf(Name::self_())
    }

    /// Returns the ident of the node if it is a parent node.
    fn parent_ident(&self) -> Option<&Ident> {
        match self {
            Node::Parent { ident, .. } => Some(ident),
            _ => None,
        }
    }

    /// Returns the ident of the node if it is a leaf node with an ident for a name.
    fn leaf_ident(&self) -> Option<&Ident> {
        match self {
            Node::Leaf(Name::Ident(ident)) => Some(ident),
            _ => None,
        }
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
        let mut cur = self.tree.root.clone();
        for segment in &self.current_path {
            cur = match cur {
                GroupOrNode::Group(group) => {
                    GroupOrNode::Node(group.find_child_by_ident(segment).unwrap_or_else(|| {
                        let child = Node::Parent {
                            ident: segment.clone(),
                            child: Rc::new(RefCell::new(GroupOrNode::Group(Group::default()))),
                        };
                        group.push_child(child.clone());
                        child
                    }))
                }
                GroupOrNode::Node(ref mut node) if node.leaf_ident() == Some(segment) => {
                    let child = Rc::new(RefCell::new(GroupOrNode::Group(Group::new_with_self())));
                    *node = Node::Parent {
                        ident: segment.clone(),
                        child,
                    };
                    GroupOrNode::Node(node.clone())
                }
                GroupOrNode::Node(Node::Parent { ident, child }) => {
                    match &*RefCell::borrow(&child) {
                        GroupOrNode::Group(group) => GroupOrNode::Node(
                            group.find_child_by_ident(segment).unwrap_or_else(|| {
                                let child = Node::Parent {
                                    ident: segment.clone(),
                                    child: Rc::new(RefCell::new(GroupOrNode::Group(
                                        Group::default(),
                                    ))),
                                };
                                group.push_child(child.clone());
                                child
                            }),
                        ),
                        _ => {
                            todo!();
                        }
                    }
                }
                GroupOrNode::Node(node) => {
                    todo!();
                }
            }
        }

        match dbg!(cur) {
            GroupOrNode::Group(group) => todo!(),
            GroupOrNode::Node(node) => match node {
                Node::Parent { child, .. } => match &*RefCell::borrow(&child) {
                    GroupOrNode::Group(group) => {
                        if let Some(existing_group) = name.as_ident().and_then(|ident| {
                            group.children().iter().find_map(|child| match child {
                                Node::Parent {
                                    ident: parent_ident,
                                    child,
                                } if parent_ident == ident => match &*RefCell::borrow(child) {
                                    GroupOrNode::Group(group) => Some(group.clone()),
                                    _ => None,
                                },
                                _ => None,
                            })
                        }) {
                            existing_group.push_child(Node::self_leaf())
                        } else {
                            group.push_child(Node::Leaf(name));
                        }
                    }
                    GroupOrNode::Node(node) => todo!(),
                },
                Node::Leaf(name) => todo!(),
            },
        }
    }
}

impl FromIterator<UseTree> for Tree {
    fn from_iter<T: IntoIterator<Item = UseTree>>(iter: T) -> Self {
        let mut visitor = Visitor::default();
        for tree in iter {
            walk_use_tree(tree, &mut visitor);
        }
        visitor.tree
    }
}

impl Extend<UseTree> for Tree {
    fn extend<T: IntoIterator<Item = UseTree>>(&mut self, iter: T) {
        let tree = take(self);
        let mut visitor = Visitor {
            tree,
            ..Visitor::default()
        };
        for tree in iter {
            walk_use_tree(tree, &mut visitor);
        }
        *self = visitor.tree;
    }
}

impl From<UseTree> for Tree {
    fn from(value: UseTree) -> Self {
        Self::from_iter([value])
    }
}

impl From<Tree> for UseTree {
    fn from(tree: Tree) -> Self {
        // match node {
        //     Node::Ident { ident, children } => {
        //         if children.is_empty() {
        //             Self::Name(UseName { ident })
        //         } else {
        //             Self::Path(UsePath {
        //                 ident,
        //                 colon2_token: <Token![::]>::default(),
        //                 tree: Box::new(UseTree::Group(UseGroup {
        //                     brace_token: Brace::default(),
        //                     items: children.into_iter().map(UseTree::from).collect(),
        //                 })),
        //             })
        //         }
        //     }
        //     Node::Glob => Self::Glob(UseGlob {
        //         star_token: <Token![*]>::default(),
        //     }),
        //     Node::Rename { ident, rename } => Self::Rename(UseRename {
        //         ident,
        //         as_token: <Token![as]>::default(),
        //         rename,
        //     }),
        // }
        todo!();
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
            Tree::from(tree)
        }}
    }

    #[test]
    fn kaki() {
        let tree = Tree::from_iter([
            parse_quote! {
                std::a::c
            },
            parse_quote! {
                std::a
            },
        ]);
        dbg!(tree);
    }
}

//     fn ident(ident: &str) -> Ident {
//         Ident::new(ident, Span::call_site())
//     }

//     macro_rules! n {
//         ($name:path) => {
//             Node::ident(ident(stringify!($name)))
//         };
//         ($name:path, [$($node:expr),+]) => {
//             Node::Ident {
//                 ident: ident(stringify!($name)),
//                 children: vec![$($node),+],
//             }
//         };
//     }

//     #[test]
//     fn test_tree_to_node() {
//         assert_eq!(parse_node!(std::a), n!(std, [n!(a)]));

//         assert_eq!(
//             parse_node!(std::{a::b, a::c}),
//             n!(std, [n!(a, [n!(b), n!(c)])])
//         );

//         assert_eq!(
//             Node::from_iter([parse_quote!(std::{a, b::c}), parse_quote!(std::{b::{d, e}})]),
//             n!(std, [n!(a), n!(b, [n!(c), n!(d), n!(e)])])
//         );
//     }

//     #[ignore] // TODO!
//     #[test]
//     fn test_tree_to_node_with_self() {
//         assert_eq!(
//             parse_node!(std::{a, a::b, a::c}),
//             n!(std, [n!(a, [n!(self), n!(b), n!(c)])])
//         );
//     }

//     #[test]
//     fn test_node_to_tree() {
//         assert_eq!(
//             UseTree::from(n!(std, [n!(a, [n!(b), n!(c)])])),
//             parse_quote!(std::{a::{b, c}})
//         )
//     }
// }
