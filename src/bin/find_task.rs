extern crate todo;

use self::todo::connect_db;
use self::todo::models::Task;
use std::str::FromStr;

pub fn main() {
    let conn = connect_db();

    let id = i32::from_str(&std::env::args_os().nth(1).unwrap().into_string().unwrap()).unwrap();

    let task = Task::find(&conn, id);
    println!("{:?}", task);

}
