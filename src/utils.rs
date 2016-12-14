use mktemp::Temp;

use std;
use std::env::var;
use std::fs::{File, OpenOptions};
use std::io::{Read, Write};
use std::path::PathBuf;
use std::process::Command;

#[derive(Debug)]
pub enum Error {
    IO,
    Subprocess,
}

impl From<std::io::Error> for Error {
    fn from(_: std::io::Error) -> Error {
        Error::IO
    }
}

fn write_editor_template(path: PathBuf, template: &str) -> Result<(), Error> {
    let mut file = OpenOptions::new().write(true)
        .truncate(true)
        .open(&path)?;

    file.write_all(template.as_bytes())?;
    Ok(())
}

pub fn read_editor_input(template: &str) -> Result<String, Error> {
    let temp_file = Temp::new_file()?;
    let path = temp_file.to_path_buf();

    write_editor_template(path.clone(), template)?;

    let status_code = Command::new(var("EDITOR").unwrap_or(String::from("vim"))).arg(path.clone())
        .status()?;

    if status_code.success() {
        let mut file = File::open(&path)?;
        let mut s = String::new();
        file.read_to_string(&mut s)?;
        Ok(s)
    } else {
        Err(Error::Subprocess)
    }
}
