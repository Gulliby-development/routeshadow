use std::cell::RefCell;

use routeshadow::{cmd_list, error::RsError, io::Io};

const APP: &[u8] = b"# a small web app\nGET / home\nGET,POST /users users\nGET /users/me users_me\nGET /users/{id:int} users_show\nGET /users/{name} users_by_name\nGET /files/{path*} files_get\nGET /files/readme.txt files_readme\n* /health health\nGET /health health_get\nGET /{kind}/latest latest\n";

struct FakeIo {
    file: Vec<u8>,
    stdout: RefCell<Vec<String>>,
    stderr: RefCell<Vec<String>>,
}

impl FakeIo {
    fn new(file: &[u8]) -> Self {
        Self {
            file: file.to_vec(),
            stdout: RefCell::new(Vec::new()),
            stderr: RefCell::new(Vec::new()),
        }
    }
}

impl Io for FakeIo {
    fn read_stdin(&self) -> Result<Vec<u8>, String> {
        Ok(self.file.clone())
    }

    fn read_file(&self, _path: &str) -> Result<Vec<u8>, String> {
        Ok(self.file.clone())
    }

    fn out(&self, text: &str) {
        self.stdout.borrow_mut().push(String::from(text));
    }

    fn err(&self, text: &str) {
        self.stderr.borrow_mut().push(String::from(text));
    }
}

fn args(values: &[&str]) -> Vec<String> {
    values.iter().map(|value| String::from(*value)).collect()
}

fn names(file: &[u8], words: &[&str]) -> String {
    let io = FakeIo::new(file);
    assert_eq!(cmd_list::run(&args(words), &io), Ok(0));
    assert!(io.stderr.borrow().is_empty());
    let out = io.stdout.borrow();
    assert_eq!(out.len(), 1);
    out[0].clone()
}

#[test]
fn names_follow_file_order() {
    assert_eq!(
        names(APP, &["app.txt", "--names"]),
        "home\nusers\nusers_me\nusers_show\nusers_by_name\nfiles_get\nfiles_readme\nhealth\nhealth_get\nlatest\n"
    );
}

#[test]
fn names_follow_by_name_order_with_flags_in_any_place() {
    let sorted = "files_get\nfiles_readme\nhealth\nhealth_get\nhome\nlatest\nusers\nusers_by_name\nusers_me\nusers_show\n";
    assert_eq!(names(APP, &["app.txt", "--names", "--by-name"]), sorted);
    assert_eq!(names(APP, &["--by-name", "app.txt", "--names"]), sorted);
    assert_eq!(
        names(APP, &["--names", "--by-name", "app.txt", "--names"]),
        sorted
    );
}

#[test]
fn names_of_one_route_is_one_line() {
    assert_eq!(names(b"GET /a only\n", &["a.txt", "--names"]), "only\n");
}

#[test]
fn names_keep_the_bytewise_sort() {
    let routes = b"POST,GET /{x:any} z\nGET / A\nGET /a a\n";
    assert_eq!(names(routes, &["m.txt", "--names"]), "z\nA\na\n");
    assert_eq!(
        names(routes, &["m.txt", "--by-name", "--names"]),
        "A\na\nz\n"
    );
}

#[test]
fn names_read_standard_input_for_a_dash() {
    assert_eq!(
        names(b"GET /a one\nGET /b two\n", &["-", "--names"]),
        "one\ntwo\n"
    );
}

#[test]
fn names_do_not_change_the_errors() {
    let io = FakeIo::new(APP);
    assert_eq!(
        cmd_list::run(&args(&["--names"]), &io),
        Err(RsError(String::from("missing FILE")))
    );
    assert_eq!(
        cmd_list::run(&args(&["a.txt", "b.txt", "--names"]), &io),
        Err(RsError(String::from("too many arguments")))
    );
    assert_eq!(
        cmd_list::run(&args(&["a.txt", "--name"]), &io),
        Err(RsError(String::from("unknown option \"--name\"")))
    );
    assert_eq!(
        cmd_list::run(&args(&["a.txt", "--names=1"]), &io),
        Err(RsError(String::from("unknown option \"--names=1\"")))
    );
    assert!(io.stdout.borrow().is_empty());
    assert!(io.stderr.borrow().is_empty());
}

#[test]
fn the_table_stays_the_default() {
    let out = names(b"GET /a only\n", &["a.txt"]);
    assert!(out.starts_with("LINE  METHODS"));
}
