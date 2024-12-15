mod output;

use std::{
    error::Error,
    io::{stdin, Read},
};

use clap::Parser;
use user_spray::format;

use self::output::Output;

#[derive(Parser)]
#[command(version, about, long_about = None)]
struct Args {
    #[arg(long, help = "Don't pass results through rustfmt")]
    skip_rustfmt: bool,

    #[arg(last = true)]
    rustfmt_args: Vec<String>,
}

fn main() -> Result<(), Box<dyn Error>> {
    let args = Args::parse();

    let mut file = String::new();
    stdin().read_to_string(&mut file)?;

    let output = Output::new(args)?;

    format(&file, output)
}
