use std::{error::Error, fs};

use insta::{assert_snapshot, glob};

#[test]
fn format() -> Result<(), Box<dyn Error>> {
    use user_spray::format;

    glob!("inputs", "*.stdin", |path| {
        let contents = fs::read_to_string(path).unwrap();
        let mut output = vec![];
        format(&contents, &mut output).unwrap();
        let output = String::from_utf8(output).unwrap();
        assert_snapshot!(output);
    });

    Ok(())
}
