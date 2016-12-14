extern crate todo;

use self::todo::connect_db;
use self::todo::models::Task;

pub fn main() {
    let conn = connect_db();

    println!("Start searching for tasks...");
    for task in Task::all(&conn) {
        println!("{:?}", task);
    }
    println!("stop searching for tasks...");

    println!("Start searching for notes...");
    for note in Task::find_notes(&conn, 2) {
        println!("{:?}", note);
    }
    println!("stop searching for notes...");
}
