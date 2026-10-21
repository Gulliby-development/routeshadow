use std::{
    cell::{Cell, RefCell},
    collections::BTreeMap,
};

use routeshadow::{
    cmd_match::{USAGE, run},
    error::RsError,
    io::Io,
};

const APP: &str = "# a small web app\nGET / home\nGET,POST /users users\nGET /users/me users_me\nGET /users/{id:int} users_show\nGET /users/{name} users_by_name\nGET /files/{path*} files_get\nGET /files/readme.txt files_readme\n* /health health\nGET /health health_get\nGET /{kind}/latest latest";

struct FakeIo {
    stdin: Vec<u8>,
    files: RefCell<BTreeMap<String, Result<Vec<u8>, String>>>,
    stdin_reads: Cell<usize>,
    file_reads: RefCell<Vec<String>>,
    stdout: RefCell<Vec<String>>,
    stderr: RefCell<Vec<String>>,
}

impl FakeIo {
    fn new(stdin: &[u8]) -> Self {
        Self {
            stdin: stdin.to_vec(),
            files: RefCell::new(BTreeMap::new()),
            stdin_reads: Cell::new(0),
            file_reads: RefCell::new(Vec::new()),
            stdout: RefCell::new(Vec::new()),
            stderr: RefCell::new(Vec::new()),
        }
    }

    fn add_file(&self, path: &str, contents: &str) {
        self.files
            .borrow_mut()
            .insert(String::from(path), Ok(contents.as_bytes().to_vec()));
    }

    fn fail_file(&self, path: &str) {
        self.files
            .borrow_mut()
            .insert(String::from(path), Err(String::from("read failed")));
    }
}

impl Io for FakeIo {
    fn read_stdin(&self) -> Result<Vec<u8>, String> {
        self.stdin_reads
            .set(self.stdin_reads.get().saturating_add(1));
        Ok(self.stdin.clone())
    }

    fn read_file(&self, path: &str) -> Result<Vec<u8>, String> {
        self.file_reads.borrow_mut().push(String::from(path));
        match self.files.borrow().get(path) {
            Some(contents) => contents.clone(),
            None => Err(String::from("not found")),
        }
    }

    fn out(&self, text: &str) {
        self.stdout.borrow_mut().push(String::from(text));
    }

    fn err(&self, text: &str) {
        self.stderr.borrow_mut().push(String::from(text));
    }
}

fn argv(values: &[&str]) -> Vec<String> {
    values.iter().map(|value| String::from(*value)).collect()
}

fn app_io() -> FakeIo {
    let io = FakeIo::new(&[]);
    io.add_file("app.txt", APP);
    io
}

fn assert_error(io: &FakeIo, args: Vec<String>, message: &str) {
    assert_eq!(run(&args, io), Err(RsError(String::from(message))));
    assert!(io.stdout.borrow().is_empty());
    assert!(io.stderr.borrow().is_empty());
}

fn assert_silent_miss(args: Vec<String>) {
    let io = app_io();
    assert_eq!(run(&args, &io), Ok(1));
    assert!(io.stdout.borrow().is_empty());
    assert!(io.stderr.borrow().is_empty());
}

#[test]
fn usage_and_command_api_match_the_public_contract() {
    assert_eq!(USAGE, "routeshadow match FILE METHOD PATH [--all]");
    let io = app_io();
    assert_eq!(run(&argv(&["app.txt", "GET", "/users/42"]), &io), Ok(0));
}

#[test]
fn unknown_options_precede_arity_and_never_read_a_source() {
    let io = FakeIo::new(&[]);
    assert_error(&io, argv(&["--write"]), "unknown option \"--write\"");
    assert_error(&io, argv(&["--"]), "unknown option \"--\"");
    assert_error(&io, argv(&["--help"]), "unknown option \"--help\"");
    assert_error(&io, argv(&["--all=true"]), "unknown option \"--all=true\"");
    assert_eq!(io.stdin_reads.get(), 0);
    assert!(io.file_reads.borrow().is_empty());
}

#[test]
fn missing_and_excess_arguments_are_checked_before_reading() {
    let io = FakeIo::new(&[]);
    assert_error(&io, argv(&[]), "missing FILE");
    assert_error(&io, argv(&["app.txt"]), "missing METHOD");
    assert_error(&io, argv(&["app.txt", "GET"]), "missing PATH");
    assert_error(
        &io,
        argv(&["app.txt", "GET", "/", "extra"]),
        "too many arguments",
    );
    assert_error(&io, argv(&["--all", "app.txt", "GET"]), "missing PATH");
    assert_eq!(io.stdin_reads.get(), 0);
    assert!(io.file_reads.borrow().is_empty());
}

#[test]
fn complete_file_loading_precedes_method_and_path_validation() {
    let io = FakeIo::new(&[]);
    io.add_file("bad.txt", "GET /a one\nPOST /b/ two");
    assert_error(
        &io,
        argv(&["bad.txt", "*", "not-a-path"]),
        "\"bad.txt\": RS005 empty segment at 2:8",
    );
    assert_eq!(*io.file_reads.borrow(), vec![String::from("bad.txt")]);

    let later_error_io = FakeIo::new(&[]);
    later_error_io.add_file("bad.txt", "GET /a one\nPOST /b/ two");
    assert_error(
        &later_error_io,
        argv(&["bad.txt", "GET", "/a"]),
        "\"bad.txt\": RS005 empty segment at 2:8",
    );
}

#[test]
fn source_errors_propagate_for_missing_empty_and_malformed_sources() {
    let missing = FakeIo::new(&[]);
    missing.fail_file("nowhere.txt");
    assert_error(
        &missing,
        argv(&["nowhere.txt", "GET", "/"]),
        "\"nowhere.txt\": cannot read the file",
    );

    let empty = FakeIo::new(&[]);
    empty.add_file("empty.txt", "# nothing");
    assert_error(
        &empty,
        argv(&["empty.txt", "GET", "/"]),
        "\"empty.txt\": no routes",
    );

    let malformed = FakeIo::new(&[0x41, 0xFF, 0x0A]);
    assert_error(
        &malformed,
        argv(&["-", "GET", "/"]),
        "<stdin>: input is not valid UTF-8",
    );
    assert_eq!(malformed.stdin_reads.get(), 1);
    assert!(malformed.file_reads.borrow().is_empty());
}

#[test]
fn methods_are_validated_before_paths() {
    let io = app_io();
    assert_error(
        &io,
        argv(&["app.txt", "*", "not-a-path"]),
        "invalid method \"*\"",
    );
    assert_error(
        &io,
        argv(&["app.txt", "get", "not-a-path"]),
        "invalid method \"get\"",
    );
    assert_error(
        &io,
        argv(&["app.txt", "", "not-a-path"]),
        "invalid method \"\"",
    );
}

#[test]
fn paths_require_a_leading_slash_and_ascii_graphic_text() {
    let io = app_io();
    assert_error(
        &io,
        argv(&["app.txt", "GET", "users"]),
        "invalid path \"users\"",
    );
    assert_error(
        &io,
        argv(&["app.txt", "GET", "/café"]),
        "invalid path \"/caf?\"",
    );
    assert_error(&io, argv(&["app.txt", "GET", ""]), "invalid path \"\"");
    assert_error(
        &io,
        argv(&["app.txt", "GET", "/a b"]),
        "invalid path \"/a b\"",
    );
    assert_error(
        &io,
        argv(&["app.txt", "GET", "/a\n"]),
        "invalid path \"/a?\"",
    );
}

#[test]
fn app_first_hit_renders_exact_rows_in_one_output_call() {
    let io = app_io();
    assert_eq!(run(&argv(&["app.txt", "GET", "/users/42"]), &io), Ok(0));
    assert_eq!(
        *io.stdout.borrow(),
        vec![String::from(
            "route       users_show\nline        5\npattern     /users/{id:int}\nparam       id=42\n"
        )]
    );
    assert!(io.stderr.borrow().is_empty());
}

#[test]
fn all_hits_follow_the_first_hit_and_accept_options_anywhere_once() {
    let expected = String::from(
        "route       users_show\nline        5\npattern     /users/{id:int}\nparam       id=42\nthen        users_by_name (line 6)\n",
    );

    let after = app_io();
    assert_eq!(
        run(&argv(&["app.txt", "GET", "/users/42", "--all"]), &after),
        Ok(0)
    );
    assert_eq!(*after.stdout.borrow(), vec![expected.clone()]);
    assert!(after.stderr.borrow().is_empty());

    let before_and_repeated = app_io();
    assert_eq!(
        run(
            &argv(&["--all", "app.txt", "GET", "/users/42", "--all"]),
            &before_and_repeated
        ),
        Ok(0)
    );
    assert_eq!(*before_and_repeated.stdout.borrow(), vec![expected]);
    assert!(before_and_repeated.stderr.borrow().is_empty());
}

#[test]
fn app_rest_and_post_examples_render_exactly() {
    let rest = app_io();
    assert_eq!(
        run(&argv(&["app.txt", "GET", "/files/a/b.txt"]), &rest),
        Ok(0)
    );
    assert_eq!(
        *rest.stdout.borrow(),
        vec![String::from(
            "route       files_get\nline        7\npattern     /files/{path*}\nparam       path=a/b.txt\n"
        )]
    );
    assert!(rest.stderr.borrow().is_empty());

    let post = app_io();
    assert_eq!(run(&argv(&["app.txt", "POST", "/users"]), &post), Ok(0));
    assert_eq!(
        *post.stdout.borrow(),
        vec![String::from(
            "route       users\nline        3\npattern     /users\n"
        )]
    );
    assert!(post.stderr.borrow().is_empty());
}

#[test]
fn every_documented_miss_is_silent_with_or_without_all() {
    assert_silent_miss(argv(&["app.txt", "GET", "/files"]));
    assert_silent_miss(argv(&["app.txt", "GET", "/files/"]));
    assert_silent_miss(argv(&["app.txt", "DELETE", "/users"]));
    assert_silent_miss(argv(&["app.txt", "GET", "/users/"]));
    assert_silent_miss(argv(&["app.txt", "GET", "/files", "--all"]));
    assert_silent_miss(argv(&["app.txt", "GET", "/files/", "--all"]));
    assert_silent_miss(argv(&["app.txt", "DELETE", "/users", "--all"]));
    assert_silent_miss(argv(&["app.txt", "GET", "/users/", "--all"]));
}

#[test]
fn two_file_examples_keep_match_precedence_and_capture_order() {
    let io = FakeIo::new(&[]);
    io.add_file(
        "two.txt",
        "GET,POST /a/{x:slug}/{more*} deep\nGET /a/{y}/b mid\nGET /a/{z:int} num\nPOST /{p}/{tail*} wide",
    );
    assert_eq!(
        run(&argv(&["two.txt", "POST", "/a/q/b", "--all"]), &io),
        Ok(0)
    );
    assert_eq!(
        *io.stdout.borrow(),
        vec![String::from(
            "route       deep\nline        1\npattern     /a/{x:slug}/{more*}\nparam       x=q\nparam       more=b\nthen        wide (line 4)\n"
        )]
    );
    assert!(io.stderr.borrow().is_empty());

    let upper_slug = FakeIo::new(&[]);
    upper_slug.add_file(
        "two.txt",
        "GET,POST /a/{x:slug}/{more*} deep\nGET /a/{y}/b mid\nGET /a/{z:int} num\nPOST /{p}/{tail*} wide",
    );
    assert_eq!(
        run(&argv(&["two.txt", "GET", "/a/Q/b"]), &upper_slug),
        Ok(0)
    );
    assert_eq!(
        *upper_slug.stdout.borrow(),
        vec![String::from(
            "route       mid\nline        2\npattern     /a/{y}/b\nparam       y=Q\n"
        )]
    );
    assert!(upper_slug.stderr.borrow().is_empty());
}

#[test]
fn stdin_and_named_files_are_isolated_and_root_rendering_is_exact() {
    let stdin = FakeIo::new(b"GET / home");
    stdin.add_file("unused.txt", "GET /a other");
    assert_eq!(run(&argv(&["-", "GET", "/"]), &stdin), Ok(0));
    assert_eq!(stdin.stdin_reads.get(), 1);
    assert!(stdin.file_reads.borrow().is_empty());
    assert_eq!(
        *stdin.stdout.borrow(),
        vec![String::from(
            "route       home\nline        1\npattern     /\n"
        )]
    );
    assert!(stdin.stderr.borrow().is_empty());

    let named = FakeIo::new(b"GET /a other");
    named.add_file("root.txt", "GET / home");
    assert_eq!(run(&argv(&["root.txt", "GET", "/"]), &named), Ok(0));
    assert_eq!(named.stdin_reads.get(), 0);
    assert_eq!(*named.file_reads.borrow(), vec![String::from("root.txt")]);
    assert_eq!(
        *named.stdout.borrow(),
        vec![String::from(
            "route       home\nline        1\npattern     /\n"
        )]
    );
    assert!(named.stderr.borrow().is_empty());
}
