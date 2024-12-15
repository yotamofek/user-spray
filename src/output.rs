use std::{
    fmt,
    io::{self, stdout, StdoutLock, Write},
    mem::ManuallyDrop,
    process::{self, Command, Stdio},
};

use fn_formats::DisplayFmt;

use crate::Args;

pub(super) enum Output {
    Stdout(StdoutLock<'static>),
    Rustfmt {
        process: process::Child,
        stdin: ManuallyDrop<process::ChildStdin>,
    },
}

impl Drop for Output {
    fn drop(&mut self) {
        if let Self::Rustfmt { process, stdin } = self {
            // Safety: stdin will never be accessed again
            unsafe { ManuallyDrop::drop(stdin) };

            let status = process.wait().expect("Could not wait for rustfmt to exit");
            if !status.success() {
                panic!(
                    "Rustfmt exited unsuccessfully{}",
                    DisplayFmt(|f| {
                        if let Some(code) = status.code() {
                            write!(f, " (code: {code})")?;
                        }
                        Ok(())
                    })
                );
            }
        }
    }
}

impl Output {
    pub(super) fn new(
        Args {
            skip_rustfmt,
            rustfmt_args,
        }: Args,
    ) -> io::Result<Self> {
        Ok(if skip_rustfmt {
            Self::Stdout(stdout().lock())
        } else {
            let mut rustfmt = Command::new("rustfmt")
                .args(rustfmt_args)
                .stdin(Stdio::piped())
                .spawn()?;
            let stdin = rustfmt.stdin.take().unwrap();
            Self::Rustfmt {
                process: rustfmt,
                stdin: ManuallyDrop::new(stdin),
            }
        })
    }
}

impl Write for Output {
    fn write(&mut self, buf: &[u8]) -> io::Result<usize> {
        match self {
            Output::Stdout(stdout) => stdout.write(buf),
            Output::Rustfmt { stdin, .. } => stdin.write(buf),
        }
    }

    fn flush(&mut self) -> io::Result<()> {
        match self {
            Output::Stdout(stdout) => stdout.flush(),
            Output::Rustfmt { stdin, .. } => stdin.flush(),
        }
    }

    fn write_all(&mut self, buf: &[u8]) -> io::Result<()> {
        match self {
            Output::Stdout(stdout) => stdout.write_all(buf),
            Output::Rustfmt { stdin, .. } => stdin.write_all(buf),
        }
    }

    fn write_fmt(&mut self, fmt: fmt::Arguments<'_>) -> io::Result<()> {
        match self {
            Output::Stdout(stdout) => stdout.write_fmt(fmt),
            Output::Rustfmt { stdin, .. } => stdin.write_fmt(fmt),
        }
    }
}
