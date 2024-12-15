mod key;

use std::collections::HashMap;

use proc_macro2::Span;
use syn::{Ident, ItemUse, Token, UseName, UsePath};

use self::key::{LeadingColon, UseKey};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub(super) enum Category {
    Std,
    External,
    Crate,
}

impl From<&Ident> for Category {
    fn from(value: &Ident) -> Self {
        macro_rules! i {
            ($ident:ident) => {
                Ident::new(stringify!($ident), Span::call_site())
            };
        }

        macro_rules! ti {
            ($tt:tt) => {
                Ident::from(<Token![$tt]>::default())
            };
        }

        if [i!(std), i!(core), i!(alloc)].contains(value) {
            Self::Std
        } else if [ti![self], ti![super], ti![crate]].contains(value) {
            Self::Crate
        } else {
            Self::External
        }
    }
}

#[derive(Debug, Clone, Default)]
pub(super) struct UseMap(HashMap<Category, HashMap<UseKey, Vec<ItemUse>>>);

impl UseMap {
    pub(super) fn take(&mut self, category: Category) -> Vec<(UseKey, Vec<ItemUse>)> {
        let mut items = self
            .0
            .remove(&category)
            .unwrap_or_default()
            .into_iter()
            .collect::<Vec<_>>();
        items.sort_by(|(key, _), (other_key, _)| key.cmp(other_key));
        items
    }
}

impl Extend<ItemUse> for UseMap {
    fn extend<T: IntoIterator<Item = ItemUse>>(&mut self, iter: T) {
        for item in iter {
            // TODO: handle comments
            assert!(item.attrs.is_empty());

            let key = UseKey {
                vis: item.vis.clone(),
                leading_colon: LeadingColon::from(item.leading_colon),
                ident: match &item.tree {
                    syn::UseTree::Path(UsePath { ident, .. }) => ident,
                    syn::UseTree::Name(UseName { ident }) => ident,
                    syn::UseTree::Rename(use_rename) => todo!(),
                    syn::UseTree::Glob(use_glob) => todo!(),
                    syn::UseTree::Group(use_group) => unreachable!(),
                }
                .clone(),
            };

            let category = Category::from(&key.ident);

            self.0
                .entry(category)
                .or_default()
                .entry(key)
                .or_default()
                .push(item);
        }
    }
}

impl FromIterator<ItemUse> for UseMap {
    fn from_iter<T: IntoIterator<Item = ItemUse>>(iter: T) -> Self {
        let mut map = Self::default();
        map.extend(iter);
        map
    }
}
