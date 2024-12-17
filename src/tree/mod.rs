mod walk;

use std::{
    borrow::Borrow,
    cell::{Cell, Ref, RefCell},
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
    Group(Rc<RefCell<Group>>),
    Node(Rc<RefCell<Node>>),
}

impl Debug for GroupOrNode {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Group(group) => RefCell::borrow(group).fmt(f),
            Self::Node(node) => RefCell::borrow(node).fmt(f),
        }
    }
}

impl From<Group> for GroupOrNode {
    fn from(group: Group) -> Self {
        Self::Group(Rc::new(RefCell::new(group)))
    }
}

impl From<Node> for GroupOrNode {
    fn from(node: Node) -> Self {
        Self::Node(Rc::new(RefCell::new(node)))
    }
}

#[derive(Clone, Default, PartialEq, Eq)]
pub(super) struct Group {
    children: Vec<Rc<RefCell<Node>>>,
}

impl Debug for Group {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_tuple("Group")
            .field(&DebugFmt(|f| {
                f.debug_list()
                    .entries(self.children.iter().map(|child| RefCell::borrow(child)))
                    .finish()
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
            root: GroupOrNode::Group(Rc::new(RefCell::new(Group::default()))),
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
            children: vec![Rc::new(RefCell::new(Node::self_leaf()))],
        }
    }

    fn find_child_by_ident(&self, ident: &Ident) -> Option<Rc<RefCell<Node>>> {
        self.children
            .iter()
            .find(|child| RefCell::borrow(child).parent_ident() == Some(ident))
            .cloned()
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
            match cur {
                GroupOrNode::Group(group) => {
                    let existing_child = {
                        let group = RefCell::borrow(&group);
                        group.find_child_by_ident(segment)
                    };
                    if let Some(child) = existing_child {
                        cur = GroupOrNode::Node(child);
                    } else {
                        let child = Rc::new(RefCell::new(Node::Parent {
                            ident: segment.clone(),
                            child: Rc::new(RefCell::new(GroupOrNode::Group(Rc::new(
                                RefCell::new(Group::default()),
                            )))),
                        }));
                        group.borrow_mut().children.push(child.clone());
                        cur = GroupOrNode::Node(child);
                    }
                }
                GroupOrNode::Node(node) => {
                    if RefCell::borrow(&node).leaf_ident() == Some(segment) {
                        let child =
                            Rc::new(RefCell::new(GroupOrNode::from(Group::new_with_self())));
                        *node.borrow_mut() = Node::Parent {
                            ident: segment.clone(),
                            child,
                        };
                        cur = GroupOrNode::Node(node);
                    } else if let Node::Parent { ident, child } = &*RefCell::borrow(&node) {
                        if let GroupOrNode::Group(group) = &*RefCell::borrow(child) {
                            if let Some(existing_child) = {
                                let group = RefCell::borrow(group);
                                group.find_child_by_ident(segment)
                            } {
                                cur = GroupOrNode::Node(existing_child);
                            } else {
                                let child = Rc::new(RefCell::new(Node::Parent {
                                    ident: segment.clone(),
                                    child: Rc::new(RefCell::new(GroupOrNode::Group(Rc::new(
                                        RefCell::new(Group::default()),
                                    )))),
                                }));
                                RefCell::borrow_mut(group).children.push(child.clone());
                                cur = GroupOrNode::Node(child);
                            }
                        } else {
                            todo!();
                        }
                    } else {
                        todo!();
                    }
                }
            }
        }

        match dbg!(cur) {
            GroupOrNode::Group(group) => todo!(),
            GroupOrNode::Node(node) => match &*RefCell::borrow(&node) {
                Node::Parent { child, .. } => match &*RefCell::borrow(child) {
                    GroupOrNode::Group(group) => {
                        if let Some(existing_group) = name.as_ident().and_then(|ident| {
                            let group = RefCell::borrow(group);
                            group
                                .children
                                .iter()
                                .find_map(|child| match &*RefCell::borrow(child) {
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
                            RefCell::borrow_mut(&existing_group)
                                .children
                                .push(Rc::new(RefCell::new(Node::self_leaf())))
                        } else {
                            RefCell::borrow_mut(group)
                                .children
                                .push(Rc::new(RefCell::new(Node::Leaf(name))));
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
        let mut tree = parse_node!(std::a::b);
        tree.extend([
            parse_quote! {
                std::a::c
            },
            parse_quote! {
                std::a
            },
            parse_quote! {
                std::a::d
            },
            parse_quote! {
                std::a::c::A
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
