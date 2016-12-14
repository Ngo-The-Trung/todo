extern crate todo;

use self::todo::connect_db;
use self::todo::models::Task;

pub fn main() {
    let conn = connect_db();

    println!("Start searching for open leavds...");
    for task in Task::open_leaves(&conn) {
        println!("{:?}", task);
    }
    println!("Done");
}
