extern crate todo;

use self::todo::connect_db;
use self::todo::models::drop_tables;

pub fn main() {
    let conn = connect_db();
    drop_tables(&conn);
}
