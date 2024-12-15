mod key;

use std::collections::HashMap;

use proc_macro2::Span;
use syn::{Ident, ItemUse, Token, UseName, UsePath, UseRename};

pub(crate) use self::key::{LeadingColon, Name, UseKey};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub(super) enum Category {
    Std,
    External,
    Crate,
}

impl From<&Name> for Category {
    fn from(value: &Name) -> Self {
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

        let ident = match value {
            Name::Ident(ident) => ident,
            Name::Glob => return Self::External,
            Name::Rename { ident: from, .. } => from,
        };

        if [i!(std), i!(core), i!(alloc)].contains(ident) {
            Self::Std
        } else if [ti![self], ti![super], ti![crate]].contains(ident) {
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
                name: match &item.tree {
                    syn::UseTree::Path(UsePath { ident, .. }) => Name::Ident(ident.clone()),
                    syn::UseTree::Name(UseName { ident }) => Name::Ident(ident.clone()),
                    syn::UseTree::Rename(UseRename { ident, rename, .. }) => Name::Rename {
                        ident: ident.clone(),
                        rename: rename.clone(),
                    },
                    syn::UseTree::Glob(_) => Name::Glob,
                    syn::UseTree::Group(_) => todo!(),
                },
            };

            let category = Category::from(&key.name);

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
