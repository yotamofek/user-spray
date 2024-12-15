mod display;
mod map;
mod tree;

use std::{error::Error, io::Write};

use syn::{spanned::Spanned, Item, ItemUse, Token, UseTree};

use self::{
    display::AsDisplay,
    map::{Category, UseMap},
    tree::Node,
};

pub fn format(file: &str, mut output: impl Write) -> Result<(), Box<dyn Error>> {
    let parsed_file = syn::parse_file(file)?;

    let mut items = parsed_file.items.into_iter().peekable();
    let mut last_use_span = None;

    loop {
        while items
            .next_if(|item| !matches!(item, Item::Use(_)))
            .is_some()
        {}

        if items.peek().is_none() {
            break;
        }

        let items = items
            .by_ref()
            .map_while(|item| match item {
                Item::Use(item) => Some(item),
                _ => None,
            })
            .collect::<Vec<_>>();
        let span = items
            .iter()
            .map(Spanned::span)
            .reduce(|a, b| a.join(b).unwrap())
            .unwrap();
        let mut use_map = items.into_iter().collect::<UseMap>();

        let prev_use_span = last_use_span.replace(span);
        let preceding_byte_range = prev_use_span
            .map(|span| span.byte_range().end)
            .unwrap_or_default()..span.byte_range().start;

        write!(output, "{}", &file[preceding_byte_range])?;

        for category_map in [Category::Std, Category::External, Category::Crate]
            .map(|category| use_map.take(category))
        {
            for (key, items) in category_map {
                let tree = UseTree::from(Node::from_iter(
                    items.into_iter().map(|ItemUse { tree, .. }| tree),
                ));
                let item = ItemUse {
                    attrs: Vec::default(),
                    vis: key.vis,
                    use_token: <Token![use]>::default(),
                    leading_colon: key.leading_colon.into(),
                    tree,
                    semi_token: <Token![;]>::default(),
                };
                writeln!(output, "{}", item.as_display())?;
            }
            writeln!(output)?;
        }
    }

    write!(
        output,
        "{}",
        &file[last_use_span
            .map(|span| span.byte_range().end)
            .unwrap_or_default()..]
    )?;

    Ok(())
}
