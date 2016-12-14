extern crate todo;

use self::todo::connect_db;
use self::todo::models::create_tables;

pub fn main() {
    let conn = connect_db();
    create_tables(&conn);
}
