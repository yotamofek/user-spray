use std::{
    error::Error,
    fs,
    io::Write,
    process::{Command, Output, Stdio},
};

use insta::{assert_snapshot, glob};

fn run_rustfmt(content: &str) -> Result<String, Box<dyn Error>> {
    let mut process = Command::new("rustfmt")
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .spawn()?;
    process
        .stdin
        .as_mut()
        .unwrap()
        .write_all(content.as_bytes())?;
    let Output { status, stdout, .. } = process.wait_with_output()?;
    status
        .success()
        .then_some(())
        .ok_or("Could not run rustfmt")?;
    Ok(String::from_utf8(stdout)?)
}

#[test]
fn format() -> Result<(), Box<dyn Error>> {
    use user_spray::format;

    glob!("inputs", "*.stdin", |path| {
        let contents = fs::read_to_string(path).unwrap();
        let mut output = vec![];
        format(&contents, &mut output).unwrap();
        let output = String::from_utf8(output).unwrap();
        assert_snapshot!(output);

        let output = run_rustfmt(&output).unwrap();
        assert_snapshot!("after-rustfmt", output);
    });

    Ok(())
}
