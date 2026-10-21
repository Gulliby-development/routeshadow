use std::{cell::RefCell, collections::BTreeMap};

use routeshadow::{
    builder,
    cmd_url::{self, run},
    error::RsError,
    io::Io,
    model::Route,
    parser,
};

const APP_FIXTURE: &str = "# a small web app
GET / home
GET,POST /users users
GET /users/me users_me
GET /users/{id:int} users_show
GET /users/{name} users_by_name
GET /files/{path*} files_get
GET /files/readme.txt files_readme
* /health health
GET /health health_get
GET /{kind}/latest latest
";

const TWO_FIXTURE: &str = "GET,POST /a/{x:slug}/{more*} deep
GET /a/{y}/b mid
GET /a/{z:int} num
POST /{p}/{tail*} wide
";

struct FakeIo {
    files: BTreeMap<String, Vec<u8>>,
    stdin: Vec<u8>,
    reads: RefCell<Vec<String>>,
    outs: RefCell<Vec<String>>,
    errs: RefCell<Vec<String>>,
}

impl FakeIo {
    fn new() -> Self {
        Self {
            files: BTreeMap::from([
                (String::from("app.txt"), APP_FIXTURE.as_bytes().to_vec()),
                (String::from("two.txt"), TWO_FIXTURE.as_bytes().to_vec()),
                (
                    String::from("bad.txt"),
                    b"GET /a one\nPOST /b/ two".to_vec(),
                ),
                (String::from("empty.txt"), b"# nothing".to_vec()),
            ]),
            stdin: APP_FIXTURE.as_bytes().to_vec(),
            reads: RefCell::new(Vec::new()),
            outs: RefCell::new(Vec::new()),
            errs: RefCell::new(Vec::new()),
        }
    }
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
            .ok_or_else(|| String::from("not configured"))
    }

    fn out(&self, text: &str) {
        self.outs.borrow_mut().push(text.to_owned());
    }

    fn err(&self, text: &str) {
        self.errs.borrow_mut().push(text.to_owned());
    }
}

fn build_from(source: &str, args: &[String]) -> Result<String, RsError> {
    let routes = parser::parse(source).expect("test route must parse");
    let Some(route) = routes.first() else {
        panic!("test route must be present");
    };
    builder::build(route, args)
}

#[test]
fn public_builder_and_url_signatures_are_available() {
    let _: fn(&Route, &[String]) -> Result<String, RsError> = builder::build;
    let _: fn(&[String], &dyn Io) -> Result<i32, RsError> = cmd_url::run;

    assert_eq!(cmd_url::USAGE, "routeshadow url FILE ROUTE [KEY=VALUE...]");
}

#[test]
fn builder_renders_root_and_basic_integer_substitution() {
    assert_eq!(build_from("GET / root", &[]), Ok(String::from("/")));
    assert_eq!(
        build_from("GET /users/{id:int} users_show", &[String::from("id=42")]),
        Ok(String::from("/users/42"))
    );
}

#[test]
fn builder_accepts_long_integers_and_valid_slug_forms() {
    assert_eq!(
        build_from(
            "GET /{id:int} number",
            &[String::from("id=000123456789012345678901234567890")]
        ),
        Ok(String::from("/000123456789012345678901234567890"))
    );
    assert_eq!(
        build_from("GET /{x:slug} slug", &[String::from("x=a")]),
        Ok(String::from("/a"))
    );
    assert_eq!(
        build_from("GET /{x:slug} slug", &[String::from("x=v2")]),
        Ok(String::from("/v2"))
    );
    assert_eq!(
        build_from("GET /{x:slug} slug", &[String::from("x=a-b")]),
        Ok(String::from("/a-b"))
    );
    assert_eq!(
        build_from("GET /{x:slug} slug", &[String::from("x=2024-10-21")]),
        Ok(String::from("/2024-10-21"))
    );
}

#[test]
fn builder_preserves_literals_assignment_bytes_and_rest_segments() {
    assert_eq!(
        build_from("GET /./../{x} path", &[String::from("x=a=b%2F?#")]),
        Ok(String::from("/./../a=b%2F?#"))
    );
    assert_eq!(
        build_from("GET /files/{path*} files", &[String::from("path=a/b.txt")]),
        Ok(String::from("/files/a/b.txt"))
    );
    assert_eq!(
        build_from(
            "GET /a/{x:slug}/{more*} deep",
            &[String::from("more=x/y"), String::from("x=ab")]
        ),
        Ok(String::from("/a/ab/x/y"))
    );
}

#[test]
fn builder_checks_argument_syntax_key_repeat_and_value_in_order() {
    assert_eq!(
        build_from("GET /users/{id:int} users_show", &[String::from("42")]),
        Err(RsError(String::from("invalid argument \"42\"")))
    );
    assert_eq!(
        build_from("GET /users/{id:int} users_show", &[String::from("nid=42")]),
        Err(RsError(String::from("unknown parameter \"nid\"")))
    );
    assert_eq!(
        build_from("GET /{id:int} route", &[String::from("=42")]),
        Err(RsError(String::from("unknown parameter \"\"")))
    );
    assert_eq!(
        build_from("GET /{id:int} route", &[String::from("ID=42")]),
        Err(RsError(String::from("unknown parameter \"ID\"")))
    );
    assert_eq!(
        build_from(
            "GET /users/{id:int} users_show",
            &[String::from("id=1"), String::from("id=2")]
        ),
        Err(RsError(String::from("repeated parameter \"id\"")))
    );
    assert_eq!(
        build_from(
            "GET /{id:int} route",
            &[String::from("id=1"), String::from("id=bad")]
        ),
        Err(RsError(String::from("repeated parameter \"id\"")))
    );
    assert_eq!(
        build_from(
            "GET /{id:int} route",
            &[String::from("nid="), String::from("42")]
        ),
        Err(RsError(String::from("unknown parameter \"nid\"")))
    );
    assert_eq!(
        build_from(
            "GET /{id:int} route",
            &[String::from("id=4x"), String::from("id=1")]
        ),
        Err(RsError(String::from(
            "invalid value \"4x\" for parameter id"
        )))
    );
    assert_eq!(
        build_from("GET / route", &[String::from("x=1")]),
        Err(RsError(String::from("unknown parameter \"x\"")))
    );
    assert_eq!(
        build_from("GET /{id:int} route", &[String::from("é")]),
        Err(RsError(String::from("invalid argument \"?\"")))
    );
}

#[test]
fn builder_reports_the_first_missing_parameter_in_pattern_order() {
    assert_eq!(
        build_from("GET /users/{id:int} users_show", &[]),
        Err(RsError(String::from("missing parameter id")))
    );
    assert_eq!(
        build_from("GET /{z}/{a} route", &[]),
        Err(RsError(String::from("missing parameter z")))
    );
    assert_eq!(
        build_from("GET /{z}/{a} route", &[String::from("z=ok")]),
        Err(RsError(String::from("missing parameter a")))
    );
    assert_eq!(
        build_from("GET /{z}/{a} route", &[String::from("bad")]),
        Err(RsError(String::from("invalid argument \"bad\"")))
    );
}

#[test]
fn builder_rejects_invalid_integer_and_slug_values() {
    assert_eq!(
        build_from("GET /{id:int} route", &[String::from("id=4x")]),
        Err(RsError(String::from(
            "invalid value \"4x\" for parameter id"
        )))
    );
    assert_eq!(
        build_from("GET /{id:int} route", &[String::from("id=-1")]),
        Err(RsError(String::from(
            "invalid value \"-1\" for parameter id"
        )))
    );
    assert_eq!(
        build_from("GET /{x:slug} route", &[String::from("x=A")]),
        Err(RsError(String::from("invalid value \"A\" for parameter x")))
    );
    assert_eq!(
        build_from("GET /{x:slug} route", &[String::from("x=a--b")]),
        Err(RsError(String::from(
            "invalid value \"a--b\" for parameter x"
        )))
    );
    assert_eq!(
        build_from("GET /{x:slug} route", &[String::from("x=-a")]),
        Err(RsError(String::from(
            "invalid value \"-a\" for parameter x"
        )))
    );
    assert_eq!(
        build_from("GET /{x:slug} route", &[String::from("x=a_b")]),
        Err(RsError(String::from(
            "invalid value \"a_b\" for parameter x"
        )))
    );
}

#[test]
fn builder_rejects_empty_slash_and_non_graphic_any_or_rest_values() {
    assert_eq!(
        build_from("GET /{x} route", &[String::from("x=")]),
        Err(RsError(String::from("invalid value \"\" for parameter x")))
    );
    assert_eq!(
        build_from("GET /{x} route", &[String::from("x=a/b")]),
        Err(RsError(String::from(
            "invalid value \"a/b\" for parameter x"
        )))
    );
    assert_eq!(
        build_from("GET /{x} route", &[String::from("x=a b")]),
        Err(RsError(String::from(
            "invalid value \"a b\" for parameter x"
        )))
    );
    assert_eq!(
        build_from("GET /{x} route", &[String::from("x=é😀")]),
        Err(RsError(String::from(
            "invalid value \"??\" for parameter x"
        )))
    );
    assert_eq!(
        build_from("GET /{x} route", &[String::from("x=a\nb")]),
        Err(RsError(String::from(
            "invalid value \"a?b\" for parameter x"
        )))
    );
    assert_eq!(
        build_from("GET /files/{path*} files", &[String::from("path=")]),
        Err(RsError(String::from(
            "invalid value \"\" for parameter path"
        )))
    );
    assert_eq!(
        build_from("GET /files/{path*} files", &[String::from("path=/a")]),
        Err(RsError(String::from(
            "invalid value \"/a\" for parameter path"
        )))
    );
    assert_eq!(
        build_from("GET /files/{path*} files", &[String::from("path=a/")]),
        Err(RsError(String::from(
            "invalid value \"a/\" for parameter path"
        )))
    );
    assert_eq!(
        build_from("GET /files/{path*} files", &[String::from("path=a//b")]),
        Err(RsError(String::from(
            "invalid value \"a//b\" for parameter path"
        )))
    );
}

#[test]
fn url_command_builds_request_examples_once_and_reads_only_the_named_source() {
    let app_io = FakeIo::new();
    assert_eq!(
        run(
            &[
                String::from("app.txt"),
                String::from("users_show"),
                String::from("id=42")
            ],
            &app_io
        ),
        Ok(0)
    );
    assert_eq!(
        app_io.outs.borrow().as_slice(),
        &[String::from("/users/42\n")]
    );
    assert!(app_io.errs.borrow().is_empty());
    assert_eq!(
        app_io.reads.borrow().as_slice(),
        &[String::from("file:app.txt")]
    );

    let two_io = FakeIo::new();
    assert_eq!(
        run(
            &[
                String::from("two.txt"),
                String::from("deep"),
                String::from("more=x/y"),
                String::from("x=ab")
            ],
            &two_io
        ),
        Ok(0)
    );
    assert_eq!(
        two_io.outs.borrow().as_slice(),
        &[String::from("/a/ab/x/y\n")]
    );
    assert!(two_io.errs.borrow().is_empty());
    assert_eq!(
        two_io.reads.borrow().as_slice(),
        &[String::from("file:two.txt")]
    );

    let home_io = FakeIo::new();
    assert_eq!(
        run(&[String::from("app.txt"), String::from("home")], &home_io),
        Ok(0)
    );
    assert_eq!(home_io.outs.borrow().as_slice(), &[String::from("/\n")]);
    assert!(home_io.errs.borrow().is_empty());
    assert_eq!(
        home_io.reads.borrow().as_slice(),
        &[String::from("file:app.txt")]
    );

    let stdin_io = FakeIo::new();
    assert_eq!(
        run(&[String::from("-"), String::from("home")], &stdin_io),
        Ok(0)
    );
    assert_eq!(stdin_io.outs.borrow().as_slice(), &[String::from("/\n")]);
    assert!(stdin_io.errs.borrow().is_empty());
    assert_eq!(stdin_io.reads.borrow().as_slice(), &[String::from("stdin")]);
}

#[test]
fn url_command_rejects_options_and_missing_arguments_before_reading() {
    let option_io = FakeIo::new();
    assert_eq!(
        run(&[String::from("--help")], &option_io),
        Err(RsError(String::from("unknown option \"--help\"")))
    );
    assert!(option_io.reads.borrow().is_empty());
    assert!(option_io.outs.borrow().is_empty());
    assert!(option_io.errs.borrow().is_empty());

    let file_io = FakeIo::new();
    assert_eq!(
        run(&[], &file_io),
        Err(RsError(String::from("missing FILE")))
    );
    assert!(file_io.reads.borrow().is_empty());

    let route_io = FakeIo::new();
    assert_eq!(
        run(&[String::from("app.txt")], &route_io),
        Err(RsError(String::from("missing ROUTE")))
    );
    assert!(route_io.reads.borrow().is_empty());

    let later_option_io = FakeIo::new();
    assert_eq!(
        run(
            &[
                String::from("app.txt"),
                String::from("nope"),
                String::from("--write")
            ],
            &later_option_io
        ),
        Err(RsError(String::from("unknown option \"--write\"")))
    );
    assert!(later_option_io.reads.borrow().is_empty());
    assert!(later_option_io.outs.borrow().is_empty());
    assert!(later_option_io.errs.borrow().is_empty());
}

#[test]
fn url_command_propagates_read_parse_and_empty_source_errors_without_writes() {
    let missing_io = FakeIo::new();
    assert_eq!(
        run(
            &[
                String::from("nowhere.txt"),
                String::from("nope"),
                String::from("42")
            ],
            &missing_io
        ),
        Err(RsError(String::from(
            "\"nowhere.txt\": cannot read the file"
        )))
    );
    assert_eq!(
        missing_io.reads.borrow().as_slice(),
        &[String::from("file:nowhere.txt")]
    );
    assert!(missing_io.outs.borrow().is_empty());
    assert!(missing_io.errs.borrow().is_empty());

    let malformed_routes_io = FakeIo::new();
    assert_eq!(
        run(
            &[
                String::from("bad.txt"),
                String::from("nope"),
                String::from("42")
            ],
            &malformed_routes_io
        ),
        Err(RsError(String::from(
            "\"bad.txt\": RS005 empty segment at 2:8"
        )))
    );
    assert_eq!(
        malformed_routes_io.reads.borrow().as_slice(),
        &[String::from("file:bad.txt")]
    );
    assert!(malformed_routes_io.outs.borrow().is_empty());
    assert!(malformed_routes_io.errs.borrow().is_empty());

    let empty_io = FakeIo::new();
    assert_eq!(
        run(
            &[String::from("empty.txt"), String::from("nope")],
            &empty_io
        ),
        Err(RsError(String::from("\"empty.txt\": no routes")))
    );
    assert_eq!(
        empty_io.reads.borrow().as_slice(),
        &[String::from("file:empty.txt")]
    );
    assert!(empty_io.outs.borrow().is_empty());
    assert!(empty_io.errs.borrow().is_empty());
}

#[test]
fn url_command_decodes_malformed_stdin_bytes_through_the_loader() {
    let mut io = FakeIo::new();
    io.stdin = vec![0x41, 0xFF, 0x0A];

    assert_eq!(
        run(&[String::from("-"), String::from("nope")], &io),
        Err(RsError(String::from("<stdin>: input is not valid UTF-8")))
    );
    assert_eq!(io.reads.borrow().as_slice(), &[String::from("stdin")]);
    assert!(io.outs.borrow().is_empty());
    assert!(io.errs.borrow().is_empty());
}

#[test]
fn url_command_loads_before_case_sensitive_route_lookup() {
    let unknown_io = FakeIo::new();
    assert_eq!(
        run(
            &[
                String::from("app.txt"),
                String::from("nope"),
                String::from("42")
            ],
            &unknown_io
        ),
        Err(RsError(String::from("unknown route \"nope\"")))
    );
    assert_eq!(
        unknown_io.reads.borrow().as_slice(),
        &[String::from("file:app.txt")]
    );
    assert!(unknown_io.outs.borrow().is_empty());
    assert!(unknown_io.errs.borrow().is_empty());

    let case_io = FakeIo::new();
    assert_eq!(
        run(&[String::from("app.txt"), String::from("Home")], &case_io),
        Err(RsError(String::from("unknown route \"Home\"")))
    );
    assert_eq!(
        case_io.reads.borrow().as_slice(),
        &[String::from("file:app.txt")]
    );
    assert!(case_io.outs.borrow().is_empty());
    assert!(case_io.errs.borrow().is_empty());
}

#[test]
fn url_command_propagates_assignment_errors_without_writes() {
    let no_equals_io = FakeIo::new();
    assert_eq!(
        run(
            &[
                String::from("app.txt"),
                String::from("users_show"),
                String::from("42")
            ],
            &no_equals_io
        ),
        Err(RsError(String::from("invalid argument \"42\"")))
    );

    let unknown_key_io = FakeIo::new();
    assert_eq!(
        run(
            &[
                String::from("app.txt"),
                String::from("users_show"),
                String::from("nid=42")
            ],
            &unknown_key_io
        ),
        Err(RsError(String::from("unknown parameter \"nid\"")))
    );

    let repeated_io = FakeIo::new();
    assert_eq!(
        run(
            &[
                String::from("app.txt"),
                String::from("users_show"),
                String::from("id=1"),
                String::from("id=2")
            ],
            &repeated_io
        ),
        Err(RsError(String::from("repeated parameter \"id\"")))
    );

    let invalid_value_io = FakeIo::new();
    assert_eq!(
        run(
            &[
                String::from("app.txt"),
                String::from("users_show"),
                String::from("id=4x")
            ],
            &invalid_value_io
        ),
        Err(RsError(String::from(
            "invalid value \"4x\" for parameter id"
        )))
    );

    let missing_value_io = FakeIo::new();
    assert_eq!(
        run(
            &[String::from("app.txt"), String::from("users_show")],
            &missing_value_io
        ),
        Err(RsError(String::from("missing parameter id")))
    );

    let invalid_rest_io = FakeIo::new();
    assert_eq!(
        run(
            &[
                String::from("app.txt"),
                String::from("files_get"),
                String::from("path=a//b")
            ],
            &invalid_rest_io
        ),
        Err(RsError(String::from(
            "invalid value \"a//b\" for parameter path"
        )))
    );

    assert!(no_equals_io.outs.borrow().is_empty());
    assert!(no_equals_io.errs.borrow().is_empty());
    assert!(unknown_key_io.outs.borrow().is_empty());
    assert!(unknown_key_io.errs.borrow().is_empty());
    assert!(repeated_io.outs.borrow().is_empty());
    assert!(repeated_io.errs.borrow().is_empty());
    assert!(invalid_value_io.outs.borrow().is_empty());
    assert!(invalid_value_io.errs.borrow().is_empty());
    assert!(missing_value_io.outs.borrow().is_empty());
    assert!(missing_value_io.errs.borrow().is_empty());
    assert!(invalid_rest_io.outs.borrow().is_empty());
    assert!(invalid_rest_io.errs.borrow().is_empty());
}
