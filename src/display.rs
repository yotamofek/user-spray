use std::fmt::{self, Display};

use fn_formats::DisplayFmt;
use syn::{
    Ident, ItemUse, Path, Token, UseGlob, UseGroup, UseName, UsePath, UseRename, UseTree,
    VisRestricted, Visibility,
};

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

impl AsDisplay for UseRename {
    fn as_display(&self) -> impl fmt::Display {
        DisplayFmt(|f| write!(f, "{} as {}", self.ident, self.rename))
    }
}

impl AsDisplay for UseGlob {
    fn as_display(&self) -> impl fmt::Display {
        "*"
    }
}

fn braced<'t, T: Display + 't>(t: T) -> impl Display + 't {
    DisplayFmt(move |f| write!(f, "{{{t}}}"))
}

fn punctuated<'a, I: IntoIterator<Item = &'a T>, T: AsDisplay + 'a>(
    items: impl Fn() -> I + 'a,
    sep: &'a str,
) -> impl Display + 'a {
    DisplayFmt(move |f| {
        let mut items = items().into_iter().peekable();
        while let Some(item) = items.next() {
            write!(f, "{}", item.as_display())?;
            if items.peek().is_some() {
                f.write_str(sep)?;
            }
        }
        Ok(())
    })
}

impl AsDisplay for UseGroup {
    fn as_display(&self) -> impl fmt::Display {
        braced(punctuated(|| self.items.iter(), ", "))
    }
}

impl AsDisplay for Ident {
    fn as_display(&self) -> impl fmt::Display {
        self
    }
}

impl AsDisplay for UseTree {
    fn as_display(&self) -> impl fmt::Display {
        DisplayFmt(move |f| match self {
            UseTree::Path(use_path) => write!(f, "{}", use_path.as_display()),
            UseTree::Name(use_name) => write!(f, "{}", use_name.as_display()),
            UseTree::Rename(use_rename) => write!(f, "{}", use_rename.as_display()),
            UseTree::Glob(use_glob) => write!(f, "{}", use_glob.as_display()),
            UseTree::Group(use_group) => write!(f, "{}", use_group.as_display()),
        })
    }
}

impl AsDisplay for Option<Token![::]> {
    fn as_display(&self) -> impl fmt::Display {
        self.map(|_| "::").unwrap_or_default()
    }
}

impl AsDisplay for Option<Token![in]> {
    fn as_display(&self) -> impl fmt::Display {
        self.map(|_| "in ").unwrap_or_default()
    }
}

impl AsDisplay for Path {
    fn as_display(&self) -> impl fmt::Display {
        DisplayFmt(|f| {
            write!(
                f,
                "{}{}",
                self.leading_colon.as_display(),
                punctuated(|| self.segments.iter().map(|segment| &segment.ident), "::")
            )
        })
    }
}

impl AsDisplay for VisRestricted {
    fn as_display(&self) -> impl fmt::Display {
        DisplayFmt(|f| {
            write!(
                f,
                "pub({}{})",
                self.in_token.as_display(),
                self.path.as_display()
            )
        })
    }
}

impl AsDisplay for Visibility {
    fn as_display(&self) -> impl fmt::Display {
        DisplayFmt(move |f| match self {
            Self::Public(_) => write!(f, "pub "),
            Self::Restricted(vis_restricted) => write!(f, "{} ", vis_restricted.as_display()),
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
