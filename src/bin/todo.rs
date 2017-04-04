extern crate todo;
extern crate clap;
extern crate mktemp;
extern crate chrono;

use std::str::FromStr;
use std::io;
use std::clone::Clone;

use self::todo::connect_db;
use self::todo::models::{Task, Note, NoteAux, Review, Template, create_tables};
use self::todo::utils::read_editor_input;

use chrono::*;
use clap::{Arg, App, SubCommand};

fn split_title_body(text: &str) -> (&str, &str) {
    // TODO Handle this failure
    let result: Vec<&str> = text.split("\n==========\n").take(2).collect();
    return (result[0], result[1]);
}
#[test]
fn test_split_title_body() {
    let str = "Hello\n==========\nWorld\n==========\nSkip this part";
    let (title, body) = split_title_body(&str);
    assert_eq!(title, "Hello");
    assert_eq!(body, "World");
}

fn template_choice(default: &str) -> String {
    let conn = connect_db();
    let templates = Template::all(&conn);

    if templates.len() == 0 {
        return default.to_owned();
    }

    println!("Enter the choice of your template:");
    let mut i = 0;
    for template in &templates {
        i += 1;
        println!("{:2} - {}", i, template.name);
    }

    let mut input = String::new();
    io::stdin()
        .read_line(&mut input)
        .expect("Cannot read from stdin");
    let index = usize::from_str(&input.trim()).expect("Cannot parse int");

    // yay for panic
    templates[index - 1].body.to_owned()
}

fn init() {
    let conn = connect_db();
    create_tables(&conn)
}

fn show_leaves() {
    let conn = connect_db();
    println!("id\tTitle");
    for task in Task::open_leaves(&conn) {
        println!("{}", task);
    }
}

use std::collections::HashMap;

fn tree(open: bool) {
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
            children_table
                .get_mut(&parent_id)
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
        if !open || (*t).open {
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

}

fn humanize_duration(seconds: f32) -> (i32, i32) {
    let hours = (seconds / 3600f32).floor();
    let minutes = ((seconds - hours * 3600f32) / 60f32).floor();
    (hours as i32, minutes as i32)
}

fn dump_notes(notes: Vec<NoteAux>) -> String {
    let mut result = "".to_string();
    for note in notes {
        let (hours, minutes) = humanize_duration(note.duration_seconds);
        result.push_str(&format!("{}: {:02} hours {:02} minutes on {}-{}-{} \n{}\n",
                                 note.id,
                                 hours,
                                 minutes,
                                 note.date_start.day(),
                                 note.date_start.month(),
                                 note.date_start.year(),
                                 note.body));
    }
    result
}

fn indent(text: String, level: i32) -> String {
    let mut prefix = "".to_string();
    for _ in 0..level {
        prefix.push_str(" ")
    }

    let mut result = "".to_string();
    let lines: Vec<&str> = text.split("\n").collect();
    for i in 0..lines.len() {
        let line = lines[i];
        result.push_str(&format!("{}{}", prefix, line));
        if i != lines.len() - 1 {
            result.push_str("\n");
        }
    }
    result
}
#[test]
fn test_indent() {
    let str = "def my_func():
    print 'xd'
    if 1 < 2:
        print 'haha'";
    let expected = "    def my_func():
        print 'xd'
        if 1 < 2:
            print 'haha'";
    let result = indent(str.to_string(), 4);
    println!("\nresult:\n{}", result);
    println!("\nexpected:\n{}", expected);
    assert_eq!(result, expected);
}

fn dump_reviews(reviews: Vec<Review>) -> String {
    let mut result = "".to_string();
    // not gonna make a hash table for this; assumes the vector is sorted by task_id
    let mut prev = -1;
    for review in reviews {
        if review.task_id != prev {
            let status = if review.open { "OPEN" } else { "DONE" };
            prev = review.task_id;
            result.push_str(&format!("\n{}: [{}] {}\n", review.task_id, status, review.task_title));
        }
        result.push_str(&indent(format!("\n{}: on {}-{}-{}\n{}\n",
                                        review.note_id,
                                        review.last_updated.day(),
                                        review.last_updated.month(),
                                        review.last_updated.year(),
                                        &indent(review.note_body, 4)),
                                4))
    }
    result
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

    println!("{}", dump_notes(notes))
}

fn new_task(parent_id: Option<i32>,
            title: Option<&str>,
            body: Option<&str>,
            template: Option<&str>) {
    let task_body = if let Some(name) = template {
        let conn = connect_db();
        Template::existing(&conn, name).unwrap()
    } else {
        template_choice("Description for your task")
    };
    let editor_title = title.unwrap_or("Title for your task");
    let editor_body = body.unwrap_or(&task_body);
    let launch_editor = !(title.is_some() && body.is_some());
    let date_created = Local::now();
    if launch_editor {
        let template = format!("{}\n==========\n{}", editor_title, editor_body);
        let input = read_editor_input(&template).expect("Failed to get user input");
        let (input_title, input_body) = split_title_body(&input);
        let conn = connect_db();
        Task::new(parent_id, input_title, input_body, date_created).create(&conn);
    } else {
        let conn = connect_db();
        Task::new(parent_id, editor_title, editor_body, date_created).create(&conn);
    };
}

fn new_note(parent_id: i32, template: Option<&str>) {
    let conn = connect_db();
    let task = Task::find(&conn, parent_id).expect("Task ID does not exist");
    let notes = Task::find_notes_aux(&conn, task.id);

    let note_body = if let Some(name) = template {
        let conn = connect_db();
        Template::existing(&conn, name).unwrap()
    } else {
        String::from("Add your note here")
    };

    let template = format!("{}\n==========\n{}\n==========\n{}",
                           &note_body,
                           task.body,
                           dump_notes(notes));
    let date_start = Local::now();
    let input = read_editor_input(&template).expect("Failed to get user input");
    let date_end = Local::now();
    let (note_body, new_task_body) = split_title_body(&input);

    let conn = connect_db();
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

fn new_template(name: &str) {
    let existing = {
        let conn = connect_db();
        Template::existing(&conn, name)
    };

    let template = existing.unwrap_or(String::from("Type your template body here"));

    let body = read_editor_input(&template).expect("Failed to get user input");
    let conn = connect_db();
    Template::upsert(&conn, name, &body);
    ()
}

fn review(days: i32) {
    let conn = connect_db();
    let reviews = Task::find_recently_updated(&conn, days);

    println!("{}", dump_reviews(reviews))
}

fn main() {
    let app = App::new("Todo list")
        .version("0.0")
        .author("Ngo The Trung <ngo.the.trung.aczne@gmail.com>")
        .about("My todo list")
        .subcommand(SubCommand::with_name("init").about("Initialize the tables"))
        .subcommand(SubCommand::with_name("new-task")
            .usage("Adds a new task entry, outputting its ID")
            .arg(Arg::with_name("parent")
                .index(1)
                .takes_value(true)
                .help("Parent task's ID"))
            .arg(Arg::with_name("title")
                .long("title")
                .takes_value(true)
                .help("A title for this new task"))
            .arg(Arg::with_name("body")
                .long("body")
                .takes_value(true)
                .help("A description for this new task"))
            .arg(Arg::with_name("template")
                .short("t")
                .long("template")
                .takes_value(true)
                .help("A template for this note's body")))
        .subcommand(SubCommand::with_name("tree")
            .about("List down all tasks in a tree format")
            .arg(Arg::with_name("open").short("o")))
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
                .help("Task's ID"))
            .arg(Arg::with_name("template")
                .short("t")
                .long("template")
                .takes_value(true)
                .help("A template for this note's body")))
        .subcommand(SubCommand::with_name("finish")
            .help("Mark a task as done")
            .arg(Arg::with_name("task")
                .index(1)
                .required(true)
                .takes_value(true)
                .help("Task's ID")))
        .subcommand(SubCommand::with_name("new-template")
            .help("Create a new template")
            .arg(Arg::with_name("name")
                .index(1)
                .required(true)
                .takes_value(true)
                .help("Template's name (unique)")))
        .subcommand(SubCommand::with_name("review")
            .help("Review recently updated & finished tasks")
            .arg(Arg::with_name("days")
                .index(1)
                .required(true)
                .takes_value(true)
                .help("Number of days into the past to search for updated/finished tasks")));

    let app_matches = app.get_matches();
    let subcommand = app_matches
        .subcommand_name()
        .expect("Please use one of the subcommands");
    let matches = app_matches.subcommand_matches(subcommand).unwrap();

    match subcommand {
        "init" => init(),
        "new-task" => {
            let parent_id_str = matches.value_of("parent");
            let parent_id = match parent_id_str {
                Some(s) => Some(i32::from_str(s).expect("Cannot cast to i32")),
                None => None,
            };
            let template = matches.value_of("template");
            new_task(parent_id,
                     matches.value_of("title"),
                     matches.value_of("body"),
                     template)
        }
        "tree" => {
            // TODO add argument to specify root node
            let open = matches.is_present("open");
            tree(open)
        }
        "view-task" => {
            let task_id_str = matches.value_of("task").unwrap();
            let task_id = i32::from_str(task_id_str).expect("Cannot cast to i32");

            view_task(task_id)
        }
        "leaves" => show_leaves(),
        "new-note" => {
            let task_id_str = matches.value_of("task").unwrap();
            let task_id = i32::from_str(task_id_str).expect("Cannot cast to i32");
            let template = matches.value_of("template");
            new_note(task_id, template);
        }
        "finish" => {
            let task_id_str = matches.value_of("task").unwrap();
            let task_id = i32::from_str(task_id_str).expect("Cannot cast to i32");
            finish(task_id);
        }
        "new-template" => {
            let name = matches.value_of("name").unwrap();
            new_template(name);
        }
        "review" => {
            let days_str = matches.value_of("days").unwrap();
            let days = i32::from_str(days_str).expect("Cannot cast to i32");
            review(days);
        }
        _ => {}
    }
}
