use std::{
    cell::{Cell, RefCell},
    collections::BTreeMap,
    io::Write,
    process::{Command, Stdio},
};

use routeshadow::{
    cli::{self, USAGE},
    io::Io,
};

const APP: &str = "# a small web app\nGET / home\nGET,POST /users users\nGET /users/me users_me\nGET /users/{id:int} users_show\nGET /users/{name} users_by_name\nGET /files/{path*} files_get\nGET /files/readme.txt files_readme\n* /health health\nGET /health health_get\nGET /{kind}/latest latest\n";
const TWO: &str = "GET,POST /a/{x:slug}/{more*} deep\nGET /a/{y}/b mid\nGET /a/{z:int} num\nPOST /{p}/{tail*} wide\n";
const BAD: &str = "GET /a one\nPOST /b/ two\n";
const EMPTY: &str = "# nothing\n";

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

fn fixture_io() -> FakeIo {
    let io = FakeIo::new(&[]);
    io.add_file("app.txt", APP);
    io.add_file("two.txt", TWO);
    io.add_file("bad.txt", BAD);
    io.add_file("empty.txt", EMPTY);
    io.fail_file("nowhere.txt");
    io
}

fn assert_cli_error(args: &[&str], message: &str) {
    let io = fixture_io();
    assert_eq!(cli::run(&argv(args), &io), 2);
    assert!(io.stdout.borrow().is_empty());
    assert_eq!(
        *io.stderr.borrow(),
        vec![format!("routeshadow: {message}\n")]
    );
}

fn assert_silent(args: &[&str]) {
    let io = fixture_io();
    assert_eq!(cli::run(&argv(args), &io), 1);
    assert!(io.stdout.borrow().is_empty());
    assert!(io.stderr.borrow().is_empty());
}

#[test]
fn sole_help_prints_the_exact_usage_once_without_reading() {
    let io = fixture_io();
    assert_eq!(cli::run(&argv(&["--help"]), &io), 0);
    assert_eq!(*io.stdout.borrow(), vec![String::from(USAGE)]);
    assert_eq!(
        USAGE,
        "usage: routeshadow check FILE\n       routeshadow list FILE [--by-name]\n       routeshadow match FILE METHOD PATH [--all]\n       routeshadow shadow FILE\n       routeshadow overlap FILE\n       routeshadow url FILE ROUTE [KEY=VALUE...]\n       routeshadow regex FILE [--named]\n       routeshadow tree FILE\n"
    );
    assert!(io.stderr.borrow().is_empty());
    assert_eq!(io.stdin_reads.get(), 0);
    assert!(io.file_reads.borrow().is_empty());
}

#[test]
fn missing_and_unknown_commands_use_the_exact_error_envelope() {
    assert_cli_error(
        &[],
        "missing command (check, list, match, shadow, overlap, url, regex or tree)",
    );
    assert_cli_error(&["fold", "x"], "unknown command \"fold\"");
    assert_cli_error(&["--help", "x"], "unknown command \"--help\"");
    assert_cli_error(&["é😀"], "unknown command \"??\"");
}

#[test]
fn option_and_arity_errors_precede_reads_and_are_rendered_once() {
    assert_cli_error(
        &["list", "app.txt", "--write"],
        "unknown option \"--write\"",
    );
    assert_cli_error(&["url", "app.txt"], "missing ROUTE");
    assert_cli_error(&["check", "a", "b"], "too many arguments");
}

#[test]
fn source_errors_and_empty_files_are_rendered_once() {
    assert_cli_error(
        &["check", "nowhere.txt"],
        "\"nowhere.txt\": cannot read the file",
    );
    assert_cli_error(&["tree", "empty.txt"], "\"empty.txt\": no routes");
    assert_cli_error(
        &["shadow", "bad.txt"],
        "\"bad.txt\": RS005 empty segment at 2:8",
    );
}

#[test]
fn match_validation_errors_preserve_order_and_ascii_sanitizing() {
    assert_cli_error(&["match", "app.txt", "*", "/x"], "invalid method \"*\"");
    assert_cli_error(
        &["match", "app.txt", "GET", "users"],
        "invalid path \"users\"",
    );
    let unicode_path = fixture_io();
    assert_eq!(
        cli::run(&argv(&["match", "app.txt", "GET", "/café"]), &unicode_path),
        2
    );
    assert!(unicode_path.stdout.borrow().is_empty());
    assert_eq!(
        *unicode_path.stderr.borrow(),
        vec![String::from("routeshadow: invalid path \"/caf?\"\n")]
    );
    assert_eq!(
        format!(
            "[{},{:?},{:?}]",
            2,
            unicode_path.stdout.borrow().join(""),
            unicode_path.stderr.borrow().join("")
        ),
        "[2,\"\",\"routeshadow: invalid path \\\"/caf?\\\"\\n\"]"
    );
    assert_cli_error(&["match", "app.txt", "GET", ""], "invalid path \"\"");
}

#[test]
fn url_errors_propagate_with_exact_messages() {
    assert_cli_error(&["url", "app.txt", "nope"], "unknown route \"nope\"");
    assert_cli_error(
        &["url", "app.txt", "users_show", "42"],
        "invalid argument \"42\"",
    );
    assert_cli_error(
        &["url", "app.txt", "users_show", "nid=42"],
        "unknown parameter \"nid\"",
    );
    assert_cli_error(
        &["url", "app.txt", "users_show", "id=1", "id=2"],
        "repeated parameter \"id\"",
    );
    assert_cli_error(
        &["url", "app.txt", "users_show", "id=4x"],
        "invalid value \"4x\" for parameter id",
    );
    assert_cli_error(
        &["url", "app.txt", "files_get", "path=a//b"],
        "invalid value \"a//b\" for parameter path",
    );
    assert_cli_error(&["url", "app.txt", "users_show"], "missing parameter id");
}

#[test]
fn check_and_list_dispatch_complete_literal_outputs() {
    let check = fixture_io();
    assert_eq!(cli::run(&argv(&["check", "app.txt"]), &check), 0);
    assert_eq!(
        *check.stdout.borrow(),
        vec![String::from(
            "ok, 10 routes\nstatic      6\ndynamic     4\nrest        1\nany-method  1\ndeepest     2\n"
        )]
    );
    assert!(check.stderr.borrow().is_empty());

    let list = fixture_io();
    assert_eq!(cli::run(&argv(&["list", "app.txt"]), &list), 0);
    assert_eq!(
        *list.stdout.borrow(),
        vec![String::from(
            "LINE  METHODS   PATTERN            NAME\n2     GET       /                  home\n3     GET,POST  /users             users\n4     GET       /users/me          users_me\n5     GET       /users/{id:int}    users_show\n6     GET       /users/{name}      users_by_name\n7     GET       /files/{path*}     files_get\n8     GET       /files/readme.txt  files_readme\n9     *         /health            health\n10    GET       /health            health_get\n11    GET       /{kind}/latest     latest\n"
        )]
    );
    assert!(list.stderr.borrow().is_empty());
}

#[test]
fn match_dispatch_preserves_capture_and_all_hit_output() {
    let io = fixture_io();
    assert_eq!(
        cli::run(
            &argv(&["match", "app.txt", "GET", "/users/42", "--all"]),
            &io
        ),
        0
    );
    assert_eq!(
        *io.stdout.borrow(),
        vec![String::from(
            "route       users_show\nline        5\npattern     /users/{id:int}\nparam       id=42\nthen        users_by_name (line 6)\n"
        )]
    );
    assert!(io.stderr.borrow().is_empty());
}

#[test]
fn relation_commands_dispatch_exact_findings_and_exit_codes() {
    let shadow = fixture_io();
    assert_eq!(cli::run(&argv(&["shadow", "app.txt"]), &shadow), 1);
    assert_eq!(
        *shadow.stdout.borrow(),
        vec![String::from(
            "files_readme (line 8) is shadowed by files_get (line 7)\nhealth_get (line 10) is shadowed by health (line 9)\n"
        )]
    );
    assert!(shadow.stderr.borrow().is_empty());

    let overlap = fixture_io();
    assert_eq!(cli::run(&argv(&["overlap", "two.txt"]), &overlap), 1);
    assert_eq!(
        *overlap.stdout.borrow(),
        vec![String::from(
            "deep (line 1) and mid (line 2) both match GET /a/a/b\ndeep (line 1) and wide (line 4) both match POST /a/a/x\n"
        )]
    );
    assert!(overlap.stderr.borrow().is_empty());
}

#[test]
fn url_and_named_regex_dispatch_exact_outputs() {
    let url = fixture_io();
    assert_eq!(
        cli::run(&argv(&["url", "app.txt", "users_show", "id=42"]), &url),
        0
    );
    assert_eq!(*url.stdout.borrow(), vec![String::from("/users/42\n")]);
    assert!(url.stderr.borrow().is_empty());

    let regex = fixture_io();
    assert_eq!(cli::run(&argv(&["regex", "app.txt", "--named"]), &regex), 0);
    assert_eq!(
        *regex.stdout.borrow(),
        vec![String::from(
            "GET ^/$ home\nGET,POST ^/users$ users\nGET ^/users/me$ users_me\nGET ^/users/(?P<id>[0-9]+)$ users_show\nGET ^/users/(?P<name>[^/]+)$ users_by_name\nGET ^/files/(?P<path>[^/]+(?:/[^/]+)*)$ files_get\nGET ^/files/readme\\.txt$ files_readme\n* ^/health$ health\nGET ^/health$ health_get\nGET ^/(?P<kind>[^/]+)/latest$ latest\n"
        )]
    );
    assert!(regex.stderr.borrow().is_empty());
}

#[test]
fn tree_dispatch_matches_the_complete_requested_example() {
    let io = fixture_io();
    assert_eq!(cli::run(&argv(&["tree", "app.txt"]), &io), 0);
    assert_eq!(
        *io.stdout.borrow(),
        vec![String::from(
            "/ = home (GET)\n  files\n    readme.txt = files_readme (GET)\n    ** = files_get (GET)\n  health = health (*), health_get (GET)\n  users = users (GET,POST)\n    me = users_me (GET)\n    :int = users_show (GET)\n    :any = users_by_name (GET)\n  :any\n    latest = latest (GET)\n"
        )]
    );
    assert!(io.stderr.borrow().is_empty());
}

#[test]
fn silent_answers_and_check_findings_have_exact_call_counts() {
    assert_silent(&["match", "app.txt", "GET", "/files"]);
    assert_silent(&["match", "app.txt", "GET", "/files/"]);
    assert_silent(&["match", "app.txt", "DELETE", "/users"]);
    assert_silent(&["match", "app.txt", "GET", "/users/"]);

    let shadow = fixture_io();
    assert_eq!(cli::run(&argv(&["shadow", "two.txt"]), &shadow), 0);
    assert!(shadow.stdout.borrow().is_empty());
    assert!(shadow.stderr.borrow().is_empty());

    let overlap = FakeIo::new(&[]);
    overlap.add_file("app.txt", "GET / home\n");
    assert_eq!(cli::run(&argv(&["overlap", "app.txt"]), &overlap), 0);
    assert!(overlap.stdout.borrow().is_empty());
    assert!(overlap.stderr.borrow().is_empty());

    let check = fixture_io();
    assert_eq!(cli::run(&argv(&["check", "bad.txt"]), &check), 1);
    assert_eq!(
        *check.stdout.borrow(),
        vec![String::from("RS005 empty segment at 2:8\n1 problem\n")]
    );
    assert!(check.stderr.borrow().is_empty());
}

#[test]
fn validation_precedence_and_selected_sources_are_preserved() {
    let option_first = fixture_io();
    assert_eq!(
        cli::run(
            &argv(&["match", "bad.txt", "*", "users", "--bad"]),
            &option_first
        ),
        2
    );
    assert!(option_first.stdout.borrow().is_empty());
    assert_eq!(
        *option_first.stderr.borrow(),
        vec![String::from("routeshadow: unknown option \"--bad\"\n")]
    );
    assert_eq!(option_first.stdin_reads.get(), 0);
    assert!(option_first.file_reads.borrow().is_empty());

    let arity_first = fixture_io();
    assert_eq!(cli::run(&argv(&["match", "bad.txt", "*"]), &arity_first), 2);
    assert!(arity_first.stdout.borrow().is_empty());
    assert_eq!(
        *arity_first.stderr.borrow(),
        vec![String::from("routeshadow: missing PATH\n")]
    );
    assert_eq!(arity_first.stdin_reads.get(), 0);
    assert!(arity_first.file_reads.borrow().is_empty());

    let load_before_validation = fixture_io();
    assert_eq!(
        cli::run(
            &argv(&["match", "bad.txt", "*", "users"]),
            &load_before_validation
        ),
        2
    );
    assert!(load_before_validation.stdout.borrow().is_empty());
    assert_eq!(
        *load_before_validation.stderr.borrow(),
        vec![String::from(
            "routeshadow: \"bad.txt\": RS005 empty segment at 2:8\n"
        )]
    );
    assert_eq!(load_before_validation.stdin_reads.get(), 0);
    assert_eq!(
        *load_before_validation.file_reads.borrow(),
        vec![String::from("bad.txt")]
    );

    let stdin = FakeIo::new(b"GET / home\n");
    stdin.add_file("unused.txt", "GET /a other\n");
    assert_eq!(cli::run(&argv(&["check", "-"]), &stdin), 0);
    assert_eq!(stdin.stdin_reads.get(), 1);
    assert!(stdin.file_reads.borrow().is_empty());
    assert_eq!(
        *stdin.stdout.borrow(),
        vec![String::from(
            "ok, 1 route\nstatic      1\ndynamic     0\nrest        0\nany-method  0\ndeepest     0\n"
        )]
    );
    assert!(stdin.stderr.borrow().is_empty());

    let named = FakeIo::new(b"GET / ignored\n");
    named.add_file("app.txt", APP);
    assert_eq!(cli::run(&argv(&["url", "app.txt", "home"]), &named), 0);
    assert_eq!(named.stdin_reads.get(), 0);
    assert_eq!(*named.file_reads.borrow(), vec![String::from("app.txt")]);
    assert_eq!(*named.stdout.borrow(), vec![String::from("/\n")]);
    assert!(named.stderr.borrow().is_empty());
}

#[test]
fn real_stdin_malformed_utf8_has_the_exact_process_diagnostic() {
    let mut child = Command::new(env!("CARGO_BIN_EXE_routeshadow"))
        .arg("check")
        .arg("-")
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .expect("failed to start routeshadow");

    let mut stdin = child.stdin.take().expect("stdin should be piped");
    stdin
        .write_all(&[0x41, 0xFF, 0x0A])
        .expect("failed to write stdin bytes");
    drop(stdin);

    let output = child
        .wait_with_output()
        .expect("failed to collect routeshadow output");
    assert_eq!(output.status.code(), Some(2));
    assert_eq!(output.stdout, b"");
    assert_eq!(
        output.stderr,
        b"routeshadow: <stdin>: input is not valid UTF-8\n"
    );
}
