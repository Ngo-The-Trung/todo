extern crate todo;
extern crate chrono;

use self::todo::connect_db;
use self::todo::models::{Task, Note};
use chrono::*;

pub fn main() {
    let conn = connect_db();

    for i in 0..10 {
        let time = Local::now();
        let parent = match i {
            0 => None,
            v => Some(i),
        };
        Task::new(parent, &format!("Task#{}", i), "Do something", time).create(&conn);
    }

    println!("Done");
}
