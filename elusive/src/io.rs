//! Various utilities

use anyhow::{bail, Context, Result};
use log::error;
use std::ffi::OsStr;
use std::path::Path;
use std::{env, fs, io};

/// Allow reading from either a file or standard input
pub enum Input {
    Stdin(io::Stdin),
    File(fs::File),
}

impl Input {
    pub fn from_path<T>(path: T) -> Result<Self>
    where
        T: AsRef<Path>,
    {
        let path = path.as_ref();

        if path == OsStr::new("-") {
            return Ok(Input::Stdin(io::stdin()));
        }

        if !path.exists() {
            error!("Input file not found: {}", path.display());
            bail!(io::Error::new(
                io::ErrorKind::NotFound,
                path.to_string_lossy()
            ));
        }

        let file = fs::File::open(path)?;
        Ok(Input::File(file))
    }
}

impl io::Read for Input {
    fn read(&mut self, buf: &mut [u8]) -> Result<usize, io::Error> {
        match self {
            Input::Stdin(stdin) => stdin.read(buf),
            Input::File(file) => file.read(buf),
        }
    }
}

pub enum Output {
    Stdout(io::Stdout),
    File(fs::File),
}

impl Output {
    pub fn from_path<T>(path: T) -> Result<Self>
    where
        T: AsRef<Path>,
    {
        let path = path.as_ref();

        if path == OsStr::new("-") {
            return Ok(Output::Stdout(io::stdout()));
        }

        let absolute = if path.is_absolute() {
            path.to_path_buf()
        } else {
            env::current_dir()?.join(path)
        };

        if !absolute
            .parent()
            .context("file has no parent directory")?
            .exists()
        {
            error!(
                "Output file parent directory does not exist: {}",
                absolute.display()
            );

            bail!(io::Error::new(
                io::ErrorKind::NotFound,
                absolute.to_string_lossy()
            ));
        }

        let file = fs::File::create(absolute)?;
        Ok(Output::File(file))
    }
}

impl io::Write for Output {
    fn write(&mut self, buf: &[u8]) -> Result<usize, io::Error> {
        match self {
            Output::Stdout(stdout) => stdout.write(buf),
            Output::File(file) => file.write(buf),
        }
    }

    fn flush(&mut self) -> Result<(), io::Error> {
        match self {
            Output::Stdout(stdout) => stdout.flush(),
            Output::File(file) => file.flush(),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_stdinout() -> Result<()> {
        assert!(matches!(Input::from_path("-")?, Input::Stdin(_)));
        assert!(matches!(Output::from_path("-")?, Output::Stdout(_)));

        Ok(())
    }
}
