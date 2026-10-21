use std::{cell::RefCell, collections::BTreeMap};

use routeshadow::{
    cmd_overlap::{USAGE as OVERLAP_USAGE, run as overlap_run},
    cmd_shadow::{USAGE as SHADOW_USAGE, run as shadow_run},
    error::RsError,
    io::Io,
};

struct FakeIo {
    stdin: Vec<u8>,
    files: BTreeMap<String, Vec<u8>>,
    reads: RefCell<Vec<String>>,
    stdout: RefCell<Vec<String>>,
    stderr: RefCell<Vec<String>>,
}

impl Io for FakeIo {
    fn read_stdin(&self) -> Result<Vec<u8>, String> {
        self.reads.borrow_mut().push(String::from("stdin"));
        Ok(self.stdin.clone())
    }

    fn read_file(&self, path: &str) -> Result<Vec<u8>, String> {
        self.reads.borrow_mut().push(format!("file:{path}"));
        self.files
            .get(path)
            .cloned()
            .ok_or_else(|| String::from("not found"))
    }

    fn out(&self, text: &str) {
        self.stdout.borrow_mut().push(String::from(text));
    }

    fn err(&self, text: &str) {
        self.stderr.borrow_mut().push(String::from(text));
    }
}

fn fake_io() -> FakeIo {
    FakeIo {
        stdin: Vec::new(),
        files: BTreeMap::new(),
        reads: RefCell::new(Vec::new()),
        stdout: RefCell::new(Vec::new()),
        stderr: RefCell::new(Vec::new()),
    }
}

fn app_io() -> FakeIo {
    let mut io = fake_io();
    io.files.insert(
        String::from("app.txt"),
        b"# a small web app
GET / home
GET,POST /users users
GET /users/me users_me
GET /users/{id:int} users_show
GET /users/{name} users_by_name
GET /files/{path*} files_get
GET /files/readme.txt files_readme
* /health health
GET /health health_get
GET /{kind}/latest latest"
            .to_vec(),
    );
    io
}

fn two_io() -> FakeIo {
    let mut io = fake_io();
    io.files.insert(
        String::from("two.txt"),
        b"GET,POST /a/{x:slug}/{more*} deep
GET /a/{y}/b mid
GET /a/{z:int} num
POST /{p}/{tail*} wide"
            .to_vec(),
    );
    io
}

#[test]
fn commands_publish_the_usage_lines_and_require_a_file() {
    assert_eq!(SHADOW_USAGE, "routeshadow shadow FILE");
    assert_eq!(OVERLAP_USAGE, "routeshadow overlap FILE");

    let io = fake_io();
    assert_eq!(
        shadow_run(&[], &io),
        Err(RsError(String::from("missing FILE")))
    );
    assert_eq!(
        overlap_run(&[], &io),
        Err(RsError(String::from("missing FILE")))
    );
    assert!(io.reads.borrow().is_empty());
    assert!(io.stdout.borrow().is_empty());
    assert!(io.stderr.borrow().is_empty());
}

#[test]
fn commands_reject_extra_arguments_and_help_as_an_unknown_option() {
    let io = fake_io();
    assert_eq!(
        shadow_run(&[String::from("a"), String::from("b")], &io),
        Err(RsError(String::from("too many arguments")))
    );
    assert_eq!(
        overlap_run(&[String::from("a"), String::from("b")], &io),
        Err(RsError(String::from("too many arguments")))
    );
    assert_eq!(
        shadow_run(&[String::from("--help")], &io),
        Err(RsError(String::from("unknown option \"--help\"")))
    );
    assert_eq!(
        overlap_run(&[String::from("--help")], &io),
        Err(RsError(String::from("unknown option \"--help\"")))
    );
    assert!(io.reads.borrow().is_empty());
    assert!(io.stdout.borrow().is_empty());
    assert!(io.stderr.borrow().is_empty());
}

#[test]
fn an_unknown_late_option_precedes_arity_and_does_not_read_a_source() {
    let io = fake_io();
    assert_eq!(
        shadow_run(
            &[
                String::from("a"),
                String::from("b"),
                String::from("--write"),
            ],
            &io
        ),
        Err(RsError(String::from("unknown option \"--write\"")))
    );
    assert_eq!(
        overlap_run(
            &[
                String::from("a"),
                String::from("b"),
                String::from("--write"),
            ],
            &io
        ),
        Err(RsError(String::from("unknown option \"--write\"")))
    );
    assert!(io.reads.borrow().is_empty());
    assert!(io.stdout.borrow().is_empty());
    assert!(io.stderr.borrow().is_empty());
}

#[test]
fn named_read_failures_are_propagated_without_output() {
    let io = fake_io();
    assert_eq!(
        shadow_run(&[String::from("nowhere.txt")], &io),
        Err(RsError(String::from(
            "\"nowhere.txt\": cannot read the file"
        )))
    );
    assert_eq!(
        overlap_run(&[String::from("nowhere.txt")], &io),
        Err(RsError(String::from(
            "\"nowhere.txt\": cannot read the file"
        )))
    );
    assert_eq!(
        io.reads.borrow().as_slice(),
        &[
            String::from("file:nowhere.txt"),
            String::from("file:nowhere.txt")
        ]
    );
    assert!(io.stdout.borrow().is_empty());
    assert!(io.stderr.borrow().is_empty());
}

#[test]
fn empty_route_files_are_errors_for_both_commands() {
    let mut io = fake_io();
    io.files
        .insert(String::from("empty.txt"), b"# nothing".to_vec());

    assert_eq!(
        shadow_run(&[String::from("empty.txt")], &io),
        Err(RsError(String::from("\"empty.txt\": no routes")))
    );
    assert_eq!(
        overlap_run(&[String::from("empty.txt")], &io),
        Err(RsError(String::from("\"empty.txt\": no routes")))
    );
    assert_eq!(
        io.reads.borrow().as_slice(),
        &[
            String::from("file:empty.txt"),
            String::from("file:empty.txt")
        ]
    );
    assert!(io.stdout.borrow().is_empty());
    assert!(io.stderr.borrow().is_empty());
}

#[test]
fn parser_errors_keep_the_file_context_and_exact_diagnostic() {
    let mut io = fake_io();
    io.files.insert(
        String::from("bad.txt"),
        b"GET /a one
POST /b/ two"
            .to_vec(),
    );

    assert_eq!(
        shadow_run(&[String::from("bad.txt")], &io),
        Err(RsError(String::from(
            "\"bad.txt\": RS005 empty segment at 2:8"
        )))
    );
    assert_eq!(
        overlap_run(&[String::from("bad.txt")], &io),
        Err(RsError(String::from(
            "\"bad.txt\": RS005 empty segment at 2:8"
        )))
    );
    assert_eq!(
        io.reads.borrow().as_slice(),
        &[String::from("file:bad.txt"), String::from("file:bad.txt")]
    );
    assert!(io.stdout.borrow().is_empty());
    assert!(io.stderr.borrow().is_empty());
}

#[test]
fn malformed_stdin_bytes_reach_the_shared_utf8_decoder() {
    let mut io = fake_io();
    io.stdin = vec![65, 255, 10];

    assert_eq!(
        shadow_run(&[String::from("-")], &io),
        Err(RsError(String::from("<stdin>: input is not valid UTF-8")))
    );
    assert_eq!(
        overlap_run(&[String::from("-")], &io),
        Err(RsError(String::from("<stdin>: input is not valid UTF-8")))
    );
    assert_eq!(
        io.reads.borrow().as_slice(),
        &[String::from("stdin"), String::from("stdin")]
    );
    assert!(io.stdout.borrow().is_empty());
    assert!(io.stderr.borrow().is_empty());
}

#[test]
fn shadow_app_reports_all_findings_in_one_lf_terminated_output_call() {
    let io = app_io();
    assert_eq!(shadow_run(&[String::from("app.txt")], &io), Ok(1));
    assert_eq!(
        io.stdout.borrow().as_slice(),
        &[String::from(
            "files_readme (line 8) is shadowed by files_get (line 7)\nhealth_get (line 10) is shadowed by health (line 9)\n"
        )]
    );
    assert!(io.stderr.borrow().is_empty());
}

#[test]
fn shadow_two_has_no_findings_and_makes_no_output_call() {
    let io = two_io();
    assert_eq!(shadow_run(&[String::from("two.txt")], &io), Ok(0));
    assert!(io.stdout.borrow().is_empty());
    assert!(io.stderr.borrow().is_empty());
}

#[test]
fn overlap_app_reports_crossings_in_one_ordered_lf_terminated_call() {
    let io = app_io();
    assert_eq!(overlap_run(&[String::from("app.txt")], &io), Ok(1));
    assert_eq!(
        io.stdout.borrow().as_slice(),
        &[String::from(
            "users_by_name (line 6) and latest (line 11) both match GET /users/latest\nfiles_get (line 7) and latest (line 11) both match GET /files/latest\n"
        )]
    );
    assert!(io.stderr.borrow().is_empty());
}

#[test]
fn overlap_two_reports_the_expected_canonical_crossings() {
    let io = two_io();
    assert_eq!(overlap_run(&[String::from("two.txt")], &io), Ok(1));
    assert_eq!(
        io.stdout.borrow().as_slice(),
        &[String::from(
            "deep (line 1) and mid (line 2) both match GET /a/a/b\ndeep (line 1) and wide (line 4) both match POST /a/a/x\n"
        )]
    );
    assert!(io.stderr.borrow().is_empty());
}

#[test]
fn stdin_is_loaded_for_each_command_and_specific_before_general_is_no_finding() {
    let mut io = fake_io();
    io.stdin = b"GET /users/me specific
GET /users/{name} general
"
    .to_vec();

    assert_eq!(shadow_run(&[String::from("-")], &io), Ok(0));
    assert_eq!(overlap_run(&[String::from("-")], &io), Ok(0));
    assert_eq!(
        io.reads.borrow().as_slice(),
        &[String::from("stdin"), String::from("stdin")]
    );
    assert!(io.stdout.borrow().is_empty());
    assert!(io.stderr.borrow().is_empty());
}
