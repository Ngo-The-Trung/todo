extern crate postgres;
extern crate mktemp;
extern crate chrono;

pub mod models;
pub mod utils;

use postgres::{Connection, TlsMode};
use std::env::var;
use std::fs::File;
use std::io::prelude::*;

pub enum Error {
    IO,
    Env,
}

impl From<std::env::VarError> for Error {
    fn from(_: std::env::VarError) -> Error {
        Error::Env
    }
}

impl From<std::io::Error> for Error {
    fn from(_: std::io::Error) -> Error {
        Error::IO
    }
}

fn read_todorc() -> Result<String, Error> {
    let home = var("HOME")?;
    let filename = format!("{}/.todorc", home);
    let mut f = File::open(filename)?;
    let mut s = String::new();
    f.read_to_string(&mut s)?;
    Ok(s.trim().to_owned())
}

pub fn connect_db() -> Connection {
    let db_url = match read_todorc() {
        Ok(url) => url,
        Err(_) => var("DATABASE_URL").expect("$DATABASE_URL is not set"),
    };
    Connection::connect(db_url, TlsMode::None).expect("Failed to establish database connection")
}
