mod walk;

use std::{
    cell::{Ref, RefCell, RefMut},
    fmt::{self, Debug},
    mem::take,
    rc::Rc,
};

use fn_formats::DebugFmt;
use syn::{token::Brace, Ident, Token, UseGlob, UseGroup, UseName, UsePath, UseRename, UseTree};

use self::walk::walk_use_tree;
use crate::{display::DebugAdapter, map::Name};

#[derive(Clone, PartialEq, Eq)]
pub(super) struct Parent {
    pub(super) ident: Ident,
    pub(super) child: Rc<RefCell<GroupOrNode>>,
}

impl Debug for Parent {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("Parent")
            .field("ident", &DebugAdapter(&self.ident))
            .field("child", &RefCell::borrow(&self.child))
            .finish()
    }
}

#[derive(Clone, PartialEq, Eq)]
pub(super) enum Node {
    Parent(Parent),
    Leaf(Name),
}

impl Debug for Node {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Parent(parent) => parent.fmt(f),
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
        self.as_parent().map(|Parent { ident, .. }| ident)
    }

    /// Returns the ident of the node if it is a leaf node with an ident for a name.
    fn leaf_ident(&self) -> Option<&Ident> {
        match self {
            Node::Leaf(Name::Ident(ident)) => Some(ident),
            _ => None,
        }
    }

    pub(super) fn as_parent(&self) -> Option<&Parent> {
        if let Self::Parent(v) = self {
            Some(v)
        } else {
            None
        }
    }

    pub(super) fn as_leaf(&self) -> Option<&Name> {
        if let Self::Leaf(v) = self {
            Some(v)
        } else {
            None
        }
    }
}

impl Visitor {
    fn descend_path_segment(mut cur: GroupOrNode, segment: &Ident) -> GroupOrNode {
        match cur {
            GroupOrNode::Group(group) => {
                GroupOrNode::Node(group.find_child_by_ident(segment).unwrap_or_else(|| {
                    let child = Node::Parent(Parent {
                        ident: segment.clone(),
                        child: Rc::new(RefCell::new(GroupOrNode::Group(Group::default()))),
                    });
                    group.push_child(child.clone());
                    child
                }))
            }
            GroupOrNode::Node(ref mut node) if node.leaf_ident() == Some(segment) => {
                todo!();
                // let child = Rc::new(RefCell::new(GroupOrNode::Group(Group::new_with_self())));
                // *node = Node::Parent(Parent {
                //     ident: segment.clone(),
                //     child,
                // });
                // GroupOrNode::Node(node.clone())
            }
            GroupOrNode::Node(Node::Parent(Parent { child, .. })) => {
                match &*RefCell::borrow(&child) {
                    GroupOrNode::Group(group) => {
                        GroupOrNode::Node(group.find_child_by_ident(segment).unwrap_or_else(|| {
                            let child = Node::Parent(Parent {
                                ident: segment.clone(),
                                child: Rc::new(RefCell::new(GroupOrNode::Group(Group::default()))),
                            });
                            group.push_child(child.clone());
                            child
                        }))
                    }
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
}

impl walk::Visitor for Visitor {
    fn enter_path(&mut self, ident: Ident) {
        self.current_path.push(ident);
    }

    fn leave_path(&mut self) {
        self.current_path.pop().unwrap();
    }

    fn visit_name(&mut self, name: Name) {
        let cur = self
            .current_path
            .iter()
            .fold(self.tree.root.clone(), |cur, segment| {
                Self::descend_path_segment(cur, segment)
            });

        match cur {
            GroupOrNode::Group(group) => {
                // TODO: check if child exists
                group.children_mut().push(Node::Leaf(name));
            }
            GroupOrNode::Node(node) => match node {
                Node::Parent(Parent { child, .. }) => match &*RefCell::borrow(&child) {
                    GroupOrNode::Group(group) => {
                        if let Some(existing_group) = name.as_ident().and_then(|ident| {
                            group
                                .children()
                                .iter()
                                .filter_map(Node::as_parent)
                                .filter(|child| child.ident == *ident)
                                .find_map(|Parent { child, .. }| match &*RefCell::borrow(child) {
                                    GroupOrNode::Group(group) => Some(group.clone()),
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

impl From<&Parent> for UseTree {
    fn from(Parent { ident, child }: &Parent) -> Self {
        Self::Path(UsePath {
            ident: ident.clone(),
            colon2_token: <Token![::]>::default(),
            tree: Box::new(Self::from(&*RefCell::borrow(child))),
        })
    }
}

impl From<&Node> for UseTree {
    fn from(node: &Node) -> Self {
        match node {
            Node::Parent(parent) => parent.into(),
            Node::Leaf(Name::Glob) => Self::Glob(UseGlob {
                star_token: <Token![*]>::default(),
            }),
            Node::Leaf(Name::Ident(ident)) => Self::Name(UseName {
                ident: ident.clone(),
            }),
            Node::Leaf(Name::Rename { ident, rename }) => Self::Rename(UseRename {
                ident: ident.clone(),
                as_token: <Token![as]>::default(),
                rename: rename.clone(),
            }),
        }
    }
}

impl From<&Group> for UseTree {
    fn from(group: &Group) -> Self {
        Self::Group(UseGroup {
            brace_token: Brace::default(),
            items: group.children().iter().map(Self::from).collect(),
        })
    }
}

impl From<&GroupOrNode> for UseTree {
    fn from(group_or_node: &GroupOrNode) -> Self {
        match group_or_node {
            GroupOrNode::Group(group) => group.into(),
            GroupOrNode::Node(node) => node.into(),
        }
    }
}

impl From<&Tree> for UseTree {
    fn from(tree: &Tree) -> Self {
        Self::from(&tree.root)
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
                std
            },
        ]);
        dbg!(UseTree::from(&tree));
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
