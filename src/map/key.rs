use std::cmp::Ordering;

use syn::{Ident, Token, Visibility};

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub(crate) enum LeadingColon {
    No,
    Yes,
}

impl From<Option<Token![::]>> for LeadingColon {
    fn from(value: Option<Token![::]>) -> Self {
        if value.is_some() {
            Self::Yes
        } else {
            Self::No
        }
    }
}

impl From<LeadingColon> for Option<Token![::]> {
    fn from(value: LeadingColon) -> Self {
        matches!(value, LeadingColon::Yes).then_some(<Token![::]>::default())
    }
}

#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub(crate) enum Name {
    Ident(Ident),
    Glob,
    Rename { ident: Ident, rename: Ident },
}

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub(crate) struct UseKey {
    pub(crate) vis: Visibility,
    pub(crate) leading_colon: LeadingColon,
    pub(crate) name: Name,
}

impl Ord for UseKey {
    fn cmp(&self, other: &Self) -> Ordering {
        macro_rules! cmp {
            ($ord:expr) => {
                match $ord {
                    Ordering::Equal => {}
                    ord => return ord,
                }
            };
            ($left:expr, $right:expr) => {
                cmp!($left.cmp($right))
            };
        }

        match (&self.vis, &other.vis) {
            (Visibility::Public(_), Visibility::Public(_)) => {}
            (Visibility::Inherited, Visibility::Inherited) => {}
            (Visibility::Restricted(vis), Visibility::Restricted(other_vis))
                if vis == other_vis => {}
            (Visibility::Public(_), _) => return Ordering::Greater,
            (Visibility::Inherited, _) => return Ordering::Less,
            (Visibility::Restricted(_), Visibility::Public(_)) => return Ordering::Less,
            (Visibility::Restricted(_), Visibility::Inherited) => return Ordering::Greater,
            (Visibility::Restricted(vis), Visibility::Restricted(other_vis)) => {
                cmp!(vis.in_token.is_some(), &other_vis.in_token.is_some());
                cmp!(
                    vis.path.leading_colon.is_some(),
                    &other_vis.path.leading_colon.is_some()
                );

                assert!([&vis.path.segments, &other_vis.path.segments]
                    .iter()
                    .copied()
                    .flatten()
                    .all(|segment| segment.arguments.is_none()));

                cmp!(vis
                    .path
                    .segments
                    .iter()
                    .map(|segment| &segment.ident)
                    .cmp(other_vis.path.segments.iter().map(|segment| &segment.ident)));
            }
        }

        cmp!(self.leading_colon, &other.leading_colon);

        self.name.cmp(&other.name)
    }
}

impl PartialOrd for UseKey {
    fn partial_cmp(&self, other: &Self) -> Option<Ordering> {
        Some(self.cmp(other))
    }
}
