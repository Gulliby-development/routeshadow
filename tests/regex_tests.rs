use std::{cell::RefCell, collections::BTreeMap};

use routeshadow::{
    cmd_regex::{self, run},
    error::RsError,
    io::Io,
    model::Route,
    parser,
    regex::regex_of,
};

struct FakeIo {
    stdin: Vec<u8>,
    files: BTreeMap<String, Result<Vec<u8>, String>>,
    reads: RefCell<Vec<String>>,
    outputs: RefCell<Vec<String>>,
    errors: RefCell<Vec<String>>,
}

impl FakeIo {
    fn new(stdin: &str) -> Self {
        Self {
            stdin: stdin.as_bytes().to_vec(),
            files: BTreeMap::new(),
            reads: RefCell::new(Vec::new()),
            outputs: RefCell::new(Vec::new()),
            errors: RefCell::new(Vec::new()),
        }
    }

    fn with_file(mut self, path: &str, text: &str) -> Self {
        self.files
            .insert(path.to_owned(), Ok(text.as_bytes().to_vec()));
        self
    }
}

impl Io for FakeIo {
    fn read_stdin(&self) -> Result<Vec<u8>, String> {
        self.reads.borrow_mut().push(String::from("<stdin>"));
        Ok(self.stdin.clone())
    }

    fn read_file(&self, path: &str) -> Result<Vec<u8>, String> {
        self.reads.borrow_mut().push(path.to_owned());
        self.files
            .get(path)
            .cloned()
            .unwrap_or_else(|| Err(String::from("not found")))
    }

    fn out(&self, text: &str) {
        self.outputs.borrow_mut().push(text.to_owned());
    }

    fn err(&self, text: &str) {
        self.errors.borrow_mut().push(text.to_owned());
    }
}

fn route(line: &str) -> Route {
    parser::parse(line)
        .expect("valid route fixture")
        .into_iter()
        .next()
        .expect("one route fixture")
}

#[test]
fn root_expression_is_anchored_in_both_modes() {
    let value = route("GET / home");
    assert_eq!(regex_of(&value, false), "^/$");
    assert_eq!(regex_of(&value, true), "^/$");
}

#[test]
fn literals_escape_every_dot_and_keep_other_valid_punctuation() {
    let value = route("GET /v1.2..3/_~- release");
    assert_eq!(regex_of(&value, false), "^/v1\\.2\\.\\.3/_~-$");
}

#[test]
fn integer_parameter_uses_the_exact_fragment() {
    let value = route("GET /{id:int} show");
    assert_eq!(regex_of(&value, false), "^/([0-9]+)$");
}

#[test]
fn slug_parameter_uses_the_exact_fragment() {
    let value = route("GET /{slug:slug} show");
    assert_eq!(regex_of(&value, false), "^/([a-z0-9]+(?:-[a-z0-9]+)*)$");
}

#[test]
fn any_parameter_uses_the_exact_fragment() {
    let value = route("GET /{value:any} show");
    assert_eq!(regex_of(&value, false), "^/([^/]+)$");
}

#[test]
fn rest_expression_works_alone_and_after_literals() {
    let alone = route("GET /{path*} files");
    let after_literal = route("GET /files/{path*} files");
    assert_eq!(regex_of(&alone, false), "^/([^/]+(?:/[^/]+)*)$");
    assert_eq!(
        regex_of(&after_literal, false),
        "^/files/([^/]+(?:/[^/]+)*)$"
    );
}

#[test]
fn mixed_captures_follow_pattern_order() {
    let value = route("GET /{first:int}/fixed/{second:any}/{tail*} mixed");
    assert_eq!(
        regex_of(&value, false),
        "^/([0-9]+)/fixed/([^/]+)/([^/]+(?:/[^/]+)*)$"
    );
    assert_eq!(
        regex_of(&value, true),
        "^/(?P<first>[0-9]+)/fixed/(?P<second>[^/]+)/(?P<tail>[^/]+(?:/[^/]+)*)$"
    );
}

#[test]
fn named_slug_and_rest_leave_noncapturing_groups_unchanged() {
    let value = route("GET /{s:slug}/{tail*} mixed");
    assert_eq!(
        regex_of(&value, true),
        "^/(?P<s>[a-z0-9]+(?:-[a-z0-9]+)*)/(?P<tail>[^/]+(?:/[^/]+)*)$"
    );
}

#[test]
fn usage_constant_is_exact() {
    assert_eq!(cmd_regex::USAGE, "routeshadow regex FILE [--named]");
}

#[test]
fn command_formats_multiple_routes_and_canonical_methods_once() {
    let io = FakeIo::new("").with_file("routes", "POST,GET /x/{id:int} one\n* / two\n");
    assert_eq!(run(&[String::from("routes")], &io), Ok(0));
    assert_eq!(
        *io.outputs.borrow(),
        vec!["GET,POST ^/x/([0-9]+)$ one\n* ^/$ two\n"]
    );
    assert!(io.errors.borrow().is_empty());
    assert_eq!(*io.reads.borrow(), vec!["routes"]);
}

#[test]
fn command_selects_stdin_and_named_file_with_repeated_interspersed_flag() {
    let stdin =
        FakeIo::new("GET /stdin from_stdin\n").with_file("routes", "GET /{id:int} from_file\n");
    assert_eq!(
        run(
            &[
                String::from("-"),
                String::from("--named"),
                String::from("--named"),
            ],
            &stdin,
        ),
        Ok(0)
    );
    assert_eq!(*stdin.reads.borrow(), vec!["<stdin>"]);
    assert_eq!(*stdin.outputs.borrow(), vec!["GET ^/stdin$ from_stdin\n"]);
    assert!(stdin.errors.borrow().is_empty());

    let named_file = FakeIo::new("").with_file("routes", "GET /{id:int} from_file\n");
    assert_eq!(
        run(
            &[String::from("--named"), String::from("routes")],
            &named_file
        ),
        Ok(0)
    );
    assert_eq!(*named_file.reads.borrow(), vec!["routes"]);
    assert_eq!(
        *named_file.outputs.borrow(),
        vec!["GET ^/(?P<id>[0-9]+)$ from_file\n"]
    );
    assert!(named_file.errors.borrow().is_empty());
}

#[test]
fn options_precede_arity_errors_and_file_reads() {
    let io = FakeIo::new("");
    assert_eq!(
        run(
            &[
                String::from("one"),
                String::from("two"),
                String::from("--unknown"),
            ],
            &io
        ),
        Err(RsError(String::from("unknown option \"--unknown\"")))
    );
    assert_eq!(*io.reads.borrow(), Vec::<String>::new());
    assert!(io.outputs.borrow().is_empty());
    assert!(io.errors.borrow().is_empty());

    let missing = FakeIo::new("");
    assert_eq!(
        run(&[], &missing),
        Err(RsError(String::from("missing FILE")))
    );
    assert!(missing.reads.borrow().is_empty());
    assert!(missing.outputs.borrow().is_empty());
    assert!(missing.errors.borrow().is_empty());

    let extra = FakeIo::new("");
    assert_eq!(
        run(&[String::from("one"), String::from("two")], &extra),
        Err(RsError(String::from("too many arguments")))
    );
    assert!(extra.reads.borrow().is_empty());
    assert!(extra.outputs.borrow().is_empty());
    assert!(extra.errors.borrow().is_empty());
}

#[test]
fn read_parse_and_empty_source_errors_do_not_write_partial_output() {
    let unreadable = FakeIo::new("");
    assert_eq!(
        run(&[String::from("missing")], &unreadable),
        Err(RsError(String::from("\"missing\": cannot read the file")))
    );
    assert_eq!(*unreadable.reads.borrow(), vec!["missing"]);
    assert!(unreadable.outputs.borrow().is_empty());
    assert!(unreadable.errors.borrow().is_empty());

    let malformed = FakeIo::new("").with_file("bad", "GET /a/ item\n");
    assert_eq!(
        run(&[String::from("bad")], &malformed),
        Err(RsError(String::from("\"bad\": RS005 empty segment at 1:7")))
    );
    assert_eq!(*malformed.reads.borrow(), vec!["bad"]);
    assert!(malformed.outputs.borrow().is_empty());
    assert!(malformed.errors.borrow().is_empty());

    let empty = FakeIo::new("").with_file("empty", "# no routes\n");
    assert_eq!(
        run(&[String::from("empty")], &empty),
        Err(RsError(String::from("\"empty\": no routes")))
    );
    assert_eq!(*empty.reads.borrow(), vec!["empty"]);
    assert!(empty.outputs.borrow().is_empty());
    assert!(empty.errors.borrow().is_empty());
}
