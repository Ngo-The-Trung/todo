extern crate todo;
extern crate clap;
extern crate mktemp;
extern crate chrono;

use std::str::FromStr;

use self::todo::connect_db;
use self::todo::models::{Task, Note};
use self::todo::utils::read_editor_input;

use chrono::*;
use clap::{Arg, App, SubCommand};

fn split_title_body(text: &str) -> (&str, &str) {
    // TODO This unwrap should be handled
    let pos = text.find("\n==========\n").unwrap();
    return (&text[0..pos], &text[pos + 12..text.len()]);
}
#[test]
fn test_split_title_body() {
    let str = "Hello\n==========\nWorld";
    let (title, body) = split_title_body(&str);
    assert_eq!(title, "Hello");
    assert_eq!(body, "World");
}

fn show_leaves() {
    let conn = connect_db();
    println!("id\tTitle");
    for task in Task::open_leaves(&conn) {
        println!("{}", task);
    }
}

use std::collections::HashMap;

fn tree() {
    let conn = connect_db();

    // TODO this assumes tasks are sorted
    let mut children_table = HashMap::new();
    let mut task_table = HashMap::new();
    let mut is_root_table = HashMap::new();

    let tasks = Task::all(&conn);

    for task in &tasks {
        if let None = children_table.get(&task.id) {
            task_table.insert(task.id, task.clone());
            children_table.insert(task.id, Vec::new());
            is_root_table.insert(task.id, true);
        }
    }

    for task in &tasks {
        if let Some(parent_id) = task.parent_id {
            let id = task.id;
            children_table.get_mut(&parent_id)
                .expect("Cannot unwrap table.get_mut()")
                .push(id);
            is_root_table.insert(id, false);
        }
    }

    struct State {
        id: i32,
        indent_level: i32,
    }

    let mut queue = Vec::new();
    for (id, _is_root) in &is_root_table {
        if *_is_root {
            queue.push(State {
                id: *id,
                indent_level: 0,
            })
        }
    }

    while queue.len() > 0 {
        let top = queue.pop().unwrap();

        let t = task_table.get(&top.id).unwrap();
        for _ in 0..top.indent_level {
            print!("    ")
        }
        println!("{}", *t);

        if let Some(children) = children_table.get(&top.id) {
            for id in children {
                queue.push(State {
                    id: *id,
                    indent_level: top.indent_level + 1,
                });
            }
        }
    }

}

fn humanize_duration(seconds: f32) -> (i32, i32) {
    let hours = (seconds / 3600f32).floor();
    let minutes = ((seconds - hours * 3600f32) / 60f32).floor();
    (hours as i32, minutes as i32)
}

fn view_task(task_id: i32) {
    let conn = connect_db();
    let task = Task::find_aux(&conn, task_id).expect("Task ID does not exist");
    let (hours, minutes) = humanize_duration(task.duration_seconds);
    println!("[{}] (accumulated: {:02} hours {:02} minutes)\n{}\n\n[Notes]",
             task.title,
             hours,
             minutes,
             task.body);

    let notes = Task::find_notes_aux(&conn, task_id);

    for note in notes {
        let (hours, minutes) = humanize_duration(note.duration_seconds);
        println!("{}: {:02} hours {:02} minutes on {}-{}-{} \n{}\n",
                 note.id,
                 hours,
                 minutes,
                 note.date_start.day(),
                 note.date_start.month(),
                 note.date_start.year(),
                 note.body);
    }
}

fn new_task(parent_id: Option<i32>, title: Option<&str>, body: Option<&str>) {
    let editor_title = title.unwrap_or("Title for your task");
    let editor_body = body.unwrap_or("Description for your task");
    let launch_editor = !(title.is_some() && body.is_some());
    let conn = connect_db();
    let date_created = Local::now();
    if launch_editor {
        let template = format!("{}\n==========\n{}", editor_title, editor_body);
        let input = read_editor_input(&template).expect("Failed to get user input");
        let (input_title, input_body) = split_title_body(&input);
        Task::new(parent_id, input_title, input_body, date_created).create(&conn);
    } else {
        Task::new(parent_id, editor_title, editor_body, date_created).create(&conn);
    };
}

fn new_note(parent_id: i32) {
    let conn = connect_db();
    let task = Task::find(&conn, parent_id).expect("Task ID does not exist");

    let template = format!("{}\n==========\n{}", task.body, "Add your note here");
    let date_start = Local::now();
    let input = read_editor_input(&template).expect("Failed to get user input");
    let date_end = Local::now();
    let (note_body, new_task_body) = split_title_body(&input);

    Note::create(&conn,
                 parent_id,
                 note_body,
                 new_task_body,
                 date_start,
                 date_end);
    ()
}

fn finish(task_id: i32) {
    let conn = connect_db();
    Task::finish(&conn, task_id);
}

fn main() {
    let app = App::new("Todo list")
        .version("0.0")
        .author("Ngo The Trung <ngo.the.trung.aczne@gmail.com>")
        .about("My todo list")
        .subcommand(SubCommand::with_name("new-task")
            .usage("Adds a new task entry, outputting its ID")
            .arg(Arg::with_name("parent")
                .index(1)
                .takes_value(true)
                .help("Parent task's ID"))
            .arg(Arg::with_name("title")
                .short("t")
                .long("title")
                .takes_value(true)
                .help("A title for this new task"))
            .arg(Arg::with_name("body")
                .short("b")
                .long("body")
                .takes_value(true)
                .help("A description for this new task")))
        .subcommand(SubCommand::with_name("tree").about("List down all tasks in a tree format"))
        .subcommand(SubCommand::with_name("view-task")
            .help("View a task's contents & metadata")
            .arg(Arg::with_name("task")
                .index(1)
                .required(true)
                .takes_value(true)
                .help("Task's ID")))
        .subcommand(SubCommand::with_name("leaves").about("List down the leaf tasks & their id"))
        .subcommand(SubCommand::with_name("new-note")
            .help("Update a task with a new note")
            .arg(Arg::with_name("task")
                .index(1)
                .required(true)
                .takes_value(true)
                .help("Task's ID")))
        .subcommand(SubCommand::with_name("finish")
            .help("Mark a task as done")
            .arg(Arg::with_name("task")
                .index(1)
                .required(true)
                .takes_value(true)
                .help("Task's ID")));

    let matches = app.get_matches();

    if let Some(matches) = matches.subcommand_matches("new-task") {
        let parent_id_str = matches.value_of("parent");
        let parent_id = match parent_id_str {
            Some(s) => Some(i32::from_str(s).expect("Cannot cast to i32")),
            None => None,
        };
        new_task(parent_id,
                 matches.value_of("title"),
                 matches.value_of("body"))
    } else if let Some(_) = matches.subcommand_matches("tree") {
        // TODO add argument to specify root node
        tree()
    } else if let Some(matches) = matches.subcommand_matches("view-task") {
        let task_id_str = matches.value_of("task").unwrap();
        let task_id = i32::from_str(task_id_str).expect("Cannot cast to i32");

        view_task(task_id)
    } else if let Some(_) = matches.subcommand_matches("leaves") {
        show_leaves()
    } else if let Some(matches) = matches.subcommand_matches("new-note") {
        let task_id_str = matches.value_of("task").unwrap();
        let task_id = i32::from_str(task_id_str).expect("Cannot cast to i32");
        new_note(task_id);
    } else if let Some(matches) = matches.subcommand_matches("finish") {
        let task_id_str = matches.value_of("task").unwrap();
        let task_id = i32::from_str(task_id_str).expect("Cannot cast to i32");
        finish(task_id);
    } else {
        println!("Please use one of the subcommands")
    }
}
