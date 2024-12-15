use std::fmt::{self, Display};

use fn_formats::DisplayFmt;
use syn::{token::PathSep, ItemUse, UseGroup, UseName, UsePath, UseTree, Visibility};

pub(super) trait AsDisplay {
    fn as_display(&self) -> impl fmt::Display;
}

impl AsDisplay for UsePath {
    fn as_display(&self) -> impl fmt::Display {
        DisplayFmt(|f| write!(f, "{}::{}", self.ident, self.tree.as_display()))
    }
}

impl AsDisplay for UseName {
    fn as_display(&self) -> impl fmt::Display {
        DisplayFmt(|f| write!(f, "{}", self.ident))
    }
}

fn braced<'t, T: Display + 't>(t: T) -> impl Display + 't {
    DisplayFmt(move |f| write!(f, "{{{t}}}"))
}

impl AsDisplay for UseGroup {
    fn as_display(&self) -> impl fmt::Display {
        braced(DisplayFmt(|f| {
            let mut items = self.items.iter().map(AsDisplay::as_display).peekable();
            while let Some(item) = items.next() {
                write!(f, "{item}")?;
                if items.peek().is_some() {
                    f.write_str(", ")?;
                }
            }
            Ok(())
        }))
    }
}

impl AsDisplay for UseTree {
    fn as_display(&self) -> impl fmt::Display {
        DisplayFmt(move |f| match self {
            UseTree::Path(use_path) => write!(f, "{}", use_path.as_display()),
            UseTree::Name(use_name) => write!(f, "{}", use_name.as_display()),
            UseTree::Rename(use_rename) => todo!(),
            UseTree::Glob(use_glob) => todo!(),
            UseTree::Group(use_group) => write!(f, "{}", use_group.as_display()),
        })
    }
}

impl AsDisplay for Option<PathSep> {
    fn as_display(&self) -> impl fmt::Display {
        self.map(|_| "::").unwrap_or_default()
    }
}

impl AsDisplay for Visibility {
    fn as_display(&self) -> impl fmt::Display {
        DisplayFmt(move |f| match self {
            Self::Public(_) => write!(f, "pub "),
            Self::Restricted(vis_restricted) => todo!(),
            Self::Inherited => Ok(()),
        })
    }
}

impl AsDisplay for ItemUse {
    fn as_display(&self) -> impl fmt::Display {
        DisplayFmt(move |f| {
            write!(
                f,
                "{}use {}{};",
                self.vis.as_display(),
                self.leading_colon.as_display(),
                self.tree.as_display()
            )
        })
    }
}
