use std;

use chrono::*;
use postgres::Connection;
use postgres::rows::Row;

#[derive(Debug, Clone)]
pub struct Task {
    pub id: i32,
    pub parent_id: Option<i32>,
    pub title: String,
    pub body: String,
    pub open: bool,
    pub date_created: DateTime<Local>,
}

#[derive(Debug, Clone)]
pub struct TaskAux {
    pub id: i32,
    pub parent_id: Option<i32>,
    pub title: String,
    pub body: String,
    pub open: bool,
    pub date_created: DateTime<Local>,
    pub duration_seconds: f32,
}

#[derive(Debug)]
pub struct Note {
    pub id: i32,
    pub task_id: i32,
    pub body: String,
    pub date_start: DateTime<Local>,
    pub date_end: DateTime<Local>,
}

#[derive(Debug)]
pub struct NoteAux {
    pub id: i32,
    pub task_id: i32,
    pub body: String,
    pub date_start: DateTime<Local>,
    pub date_end: DateTime<Local>,
    pub duration_seconds: f32,
}

pub fn create_tables(conn: &Connection) {
    Task::create_table(conn);
    Note::create_table(conn);
}

pub fn drop_tables(conn: &Connection) {
    Task::drop_table(conn);
    Note::drop_table(conn);
}

impl Task {
    pub fn create_table(conn: &Connection) {
        conn.execute("
CREATE TABLE IF NOT EXISTS task (
    id              SERIAL PRIMARY KEY,
    parent_id       INTEGER REFERENCES task(id) ON DELETE CASCADE,
    title           TEXT NOT NULL,
    body            TEXT NOT NULL,
    open            BOOL NOT NULL DEFAULT TRUE,
    date_created    TIMESTAMP WITH TIME ZONE NOT NULL
);
",
                     &[])
            .unwrap();
    }

    pub fn drop_table(conn: &Connection) {
        conn.execute("DROP TABLE IF EXISTS task CASCADE", &[]).unwrap();
    }

    pub fn new(parent_id: Option<i32>,
               title: &str,
               body: &str,
               date_created: DateTime<Local>)
               -> Self {
        Task {
            id: 0,
            parent_id: parent_id,
            title: title.to_owned(),
            body: body.to_owned(),
            open: true,
            date_created: date_created,
        }
    }

    pub fn create(self, conn: &Connection) {
        // TODO Returns the ID
        conn.execute("INSERT INTO task(parent_id, title, body, date_created) VALUES ($1, $2, $3, \
                      $4)",
                     &[&self.parent_id, &self.title, &self.body, &self.date_created])
            .unwrap();
    }

    pub fn delete(conn: &Connection, id: i32) {
        conn.execute("DELETE FROM task WHERE id = $1", &[&id])
            .unwrap();
    }

    pub fn finish(conn: &Connection, id: i32) {
        // TODO Check how many rows were affected
        conn.execute("UPDATE task SET open = FALSE WHERE id = $1", &[&id])
            .unwrap();
    }

    pub fn unpack(row: Row) -> Task {
        Task {
            id: row.get(0),
            parent_id: row.get(1),
            title: row.get(2),
            body: row.get(3),
            open: row.get(4),
            date_created: row.get(5),
        }
    }

    pub fn all(conn: &Connection) -> Vec<Task> {
        let mut result = vec![];
        for row in
            &conn.query("SELECT id, parent_id, title, body, open, date_created FROM task ORDER BY \
                        date_created DESC",
                       &[])
                .unwrap() {
            let r = &mut result;
            r.push(Task::unpack(row));
        }
        result
    }

    pub fn find(conn: &Connection, id: i32) -> Option<Task> {
        let rows =
            &conn.query("SELECT id, parent_id, title, body, open, date_created FROM task WHERE id \
                        = $1 ORDER BY date_created DESC",
                       &[&id])
                .unwrap();

        if rows.len() != 1 {
            None
        } else {
            let row = rows.get(0);
            Some(Task::unpack(row))
        }
    }

    pub fn find_aux(conn: &Connection, id: i32) -> Option<TaskAux> {
        let rows = &conn.query("
SELECT id, parent_id, title, body, open, date_created, EXTRACT(SECOND FROM \
                    duration)::REAL
FROM task, (SELECT SUM(note.date_end - note.date_start) as \
                    duration FROM note where note.task_id = id) t
WHERE id = $1 ORDER BY \
                    date_created DESC;
",
                   &[&id])
            .unwrap();

        if rows.len() != 1 {
            None
        } else {
            let row = rows.get(0);
            let duration: Option<f32> = row.get(6);
            let duration_seconds: f32 = duration.unwrap_or(0f32);
            Some(TaskAux {
                id: row.get(0),
                parent_id: row.get(1),
                title: row.get(2),
                body: row.get(3),
                open: row.get(4),
                date_created: row.get(5),
                duration_seconds: duration_seconds,
            })
        }
    }

    pub fn find_notes(conn: &Connection, id: i32) -> Vec<Note> {
        let mut result = vec![];
        for row in
            &conn.query("SELECT id, task_id, body, date_start, date_end FROM note WHERE task_id = \
                        $1 ORDER BY date_start",
                       &[&id])
                .unwrap() {
            let r = &mut result;
            r.push(Note {
                id: row.get(0),
                task_id: row.get(1),
                body: row.get(2),
                date_start: row.get(3),
                date_end: row.get(4),
            });
        }
        result
    }

    pub fn find_notes_aux(conn: &Connection, id: i32) -> Vec<NoteAux> {
        let mut result = vec![];
        for row in &conn.query("
SELECT id, task_id, body, date_start, date_end, EXTRACT(SECOND FROM \
                    date_end - date_start)::REAL as duration FROM note WHERE task_id = $1 ORDER \
                    BY date_start",
                   &[&id])
            .unwrap() {
            let r = &mut result;
            r.push(NoteAux {
                id: row.get(0),
                task_id: row.get(1),
                body: row.get(2),
                date_start: row.get(3),
                date_end: row.get(4),
                duration_seconds: row.get(5),
            });
        }
        result
    }

    pub fn notes(self, conn: &Connection) -> Vec<Note> {
        Self::find_notes(conn, self.id)
    }

    pub fn open_leaves(conn: &Connection) -> Vec<Task> {
        let mut result = vec![];
        for row in &conn.query("
SELECT id, parent_id, title, body, open, date_created from task where id \
                    not in
(SELECT DISTINCT t1.id FROM task t1, task t2 WHERE t1.id = \
                    t2.parent_id) ORDER BY date_created DESC",
                   &[])
            .unwrap() {
            let r = &mut result;
            r.push(Task::unpack(row));
        }
        result
    }
}

impl std::fmt::Display for Task {
    fn fmt(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
        write!(f, "{:3}: {}", self.id, self.title)
    }
}

impl Note {
    pub fn create_table(conn: &Connection) {
        conn.execute("
CREATE TABLE IF NOT EXISTS note (
    id          SERIAL PRIMARY KEY,
    task_id     INTEGER REFERENCES task(id) ON DELETE CASCADE,
    body        TEXT NOT NULL,
    date_start  TIMESTAMP WITH TIME ZONE NOT NULL,
    date_end    TIMESTAMP WITH TIME ZONE NOT NULL
);
    ",
                     &[])
            .unwrap();
    }

    pub fn drop_table(conn: &Connection) {
        conn.execute("DROP TABLE IF EXISTS note CASCADE", &[]).unwrap();
    }

    pub fn create(conn: &Connection,
                  task_id: i32,
                  body: &str,
                  task_body: &str,
                  date_start: DateTime<Local>,
                  date_end: DateTime<Local>) {
        let trans = conn.transaction().unwrap();

        trans.execute("INSERT INTO note(task_id, body, date_start, date_end) VALUES ($1, $2, $3, \
                      $4)",
                     &[&task_id, &body, &date_start, &date_end])
            .unwrap();
        trans.execute("UPDATE task SET body = $2 WHERE id = $1",
                     &[&task_id, &task_body])
            .unwrap();
        trans.commit().unwrap();
    }

    pub fn delete(conn: &Connection, id: i32) {
        conn.execute("DELETE FROM note WHERE id = $1", &[&id])
            .unwrap();
    }
}
