use syn::{Ident, UseGroup, UseName, UsePath, UseRename, UseTree};

use crate::map::Name;

pub(super) trait Visitor {
    fn enter_path(&mut self, ident: Ident);
    fn leave_path(&mut self);
    fn visit_name(&mut self, name: Name);
}

pub(super) fn walk_use_tree(tree: UseTree, visitor: &mut impl Visitor) {
    match tree {
        UseTree::Path(UsePath { ident, tree, .. }) => {
            visitor.enter_path(ident);
            walk_use_tree(*tree, visitor);
            visitor.leave_path();
        }
        UseTree::Name(UseName { ident }) => visitor.visit_name(Name::Ident(ident)),
        UseTree::Rename(UseRename { ident, rename, .. }) => {
            visitor.visit_name(Name::Rename { ident, rename })
        }
        UseTree::Glob(_) => visitor.visit_name(Name::Glob),
        UseTree::Group(UseGroup { items, .. }) => {
            for tree in items {
                walk_use_tree(tree, visitor);
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use proc_macro2::Span;
    use syn::{parse_quote, ItemUse};

    use super::*;

    macro_rules! parse_use_tree {
        {$($tt:tt)*} => {{
            let item: ItemUse = parse_quote! {
                $($tt)*
            };
            item.tree
        }}
    }

    fn ident(ident: &str) -> Ident {
        Ident::new(ident, Span::call_site())
    }

    #[test]
    fn test_visit() {
        #[derive(Debug, Default, Clone)]
        struct Visitor {
            current_path: Vec<Ident>,
            result: Vec<Vec<Ident>>,
        }

        impl super::Visitor for Visitor {
            fn enter_path(&mut self, ident: Ident) {
                self.current_path.push(ident);
            }

            fn leave_path(&mut self) {
                self.current_path
                    .pop()
                    .expect("trying to leave path at level 0");
            }

            fn visit_name(&mut self, name: Name) {
                let Name::Ident(ident) = name else {
                    unreachable!();
                };

                self.result.push(
                    self.current_path
                        .clone()
                        .into_iter()
                        .chain([ident])
                        .collect(),
                )
            }
        }

        fn assert_visitor_result(tree: UseTree, expected: Vec<Vec<Ident>>) {
            let mut visitor = Visitor::default();
            walk_use_tree(tree, &mut visitor);
            assert_eq!(visitor.result, expected);
        }

        macro_rules! expected {
            ($([$($path:path),+]),+) => {
                vec![$(
                    vec![$(
                        ident(stringify!($path)),
                    )+],
                )+]
            };
        }

        assert_visitor_result(
            parse_use_tree! {
                use std::{a, b::c, d::{e, f}};
            },
            expected![[std, a], [std, b, c], [std, d, e], [std, d, f]],
        );
    }
}
