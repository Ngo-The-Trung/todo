extern crate todo;
extern crate chrono;

use self::todo::connect_db;
use self::todo::models::{Task, Note};
use chrono::*;

pub fn main() {
    let now = Local::now();

    let conn = connect_db();

    conn.execute("
    CREATE TABLE IF NOT EXISTS test (
    id          SERIAL PRIMARY KEY,
    date_start  TIMESTAMP WITH TIME ZONE,
    date_end    TIMESTAMP WITH TIME ZONE
    );
    ",
                 &[])
        .unwrap();

    conn.execute("
    INSERT INTO test(date_start, date_end) VALUES($1, $2)
    ",
                 &[&now, &now])
        .unwrap();

    println!("Done");
}
