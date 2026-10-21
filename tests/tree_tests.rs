use std::cell::RefCell;

use routeshadow::{
    cmd_tree::{self, USAGE},
    error::RsError,
    io::Io,
    model::Route,
    parser, tree,
};

struct FakeIo {
    stdin: Result<Vec<u8>, String>,
    files: Vec<(String, Result<Vec<u8>, String>)>,
    reads: RefCell<Vec<String>>,
    outputs: RefCell<Vec<String>>,
    errors: RefCell<Vec<String>>,
}

impl Io for FakeIo {
    fn read_stdin(&self) -> Result<Vec<u8>, String> {
        self.reads.borrow_mut().push(String::from("<stdin>"));
        self.stdin.clone()
    }

    fn read_file(&self, path: &str) -> Result<Vec<u8>, String> {
        self.reads.borrow_mut().push(String::from(path));
        match self.files.iter().find(|(name, _)| name == path) {
            Some((_, contents)) => contents.clone(),
            None => Err(String::from("unexpected file read")),
        }
    }

    fn out(&self, text: &str) {
        self.outputs.borrow_mut().push(String::from(text));
    }

    fn err(&self, text: &str) {
        self.errors.borrow_mut().push(String::from(text));
    }
}

fn fake_io(
    stdin: Result<Vec<u8>, String>,
    files: Vec<(String, Result<Vec<u8>, String>)>,
) -> FakeIo {
    FakeIo {
        stdin,
        files,
        reads: RefCell::new(Vec::new()),
        outputs: RefCell::new(Vec::new()),
        errors: RefCell::new(Vec::new()),
    }
}

fn parse_routes(text: &str) -> Vec<Route> {
    match parser::parse(text) {
        Ok(routes) => routes,
        Err(problem) => panic!("valid fixture failed to parse: {}", problem.render()),
    }
}

#[test]
fn empty_input_renders_the_unconditional_root() {
    assert_eq!(tree::lines(&parse_routes("")), vec!["/"]);
}

#[test]
fn root_annotations_retain_route_input_order() {
    assert_eq!(
        tree::lines(&parse_routes("POST / second\nGET / first")),
        vec!["/ = second (POST), first (GET)"]
    );
}

#[test]
fn parameter_types_merge_by_type_and_keep_terminal_order() {
    assert_eq!(
        tree::lines(&parse_routes(
            "GET /{x:int}/z one\nPOST /{y:int} two\nGET /{p*} rest_one\nPOST /{q*} rest_two\nGET /{a} any_one\nPOST /{b:any} any_two"
        )),
        vec![
            "/",
            "  :int = two (POST)",
            "    z = one (GET)",
            "  :any = any_one (GET), any_two (POST)",
            "  ** = rest_one (GET), rest_two (POST)",
        ]
    );
}

#[test]
fn children_sort_bytewise_then_by_parameter_and_rest_kind() {
    assert_eq!(
        tree::lines(&parse_routes(
            "GET /{r*} rest\nGET /{a} any\nGET /{s:slug} slug\nGET /{i:int} int\nGET /a lower\nGET /Z upper\nGET /0 digit\nGET /_ under\nGET /a/z child"
        )),
        vec![
            "/",
            "  0 = digit (GET)",
            "  Z = upper (GET)",
            "  _ = under (GET)",
            "  a = lower (GET)",
            "    z = child (GET)",
            "  :int = int (GET)",
            "  :slug = slug (GET)",
            "  :any = any (GET)",
            "  ** = rest (GET)",
        ]
    );
}

#[test]
fn app_example_renders_exact_tree_lines() {
    assert_eq!(
        tree::lines(&parse_routes(
            "# a small web app\nGET / home\nGET,POST /users users\nGET /users/me users_me\nGET /users/{id:int} users_show\nGET /users/{name} users_by_name\nGET /files/{path*} files_get\nGET /files/readme.txt files_readme\n* /health health\nGET /health health_get\nGET /{kind}/latest latest"
        )),
        vec![
            "/ = home (GET)",
            "  files",
            "    readme.txt = files_readme (GET)",
            "    ** = files_get (GET)",
            "  health = health (*), health_get (GET)",
            "  users = users (GET,POST)",
            "    me = users_me (GET)",
            "    :int = users_show (GET)",
            "    :any = users_by_name (GET)",
            "  :any",
            "    latest = latest (GET)",
        ]
    );
}

#[test]
fn two_example_renders_type_order_and_depth_first_indentation() {
    assert_eq!(
        tree::lines(&parse_routes(
            "GET,POST /a/{x:slug}/{more*} deep\nGET /a/{y}/b mid\nGET /a/{z:int} num\nPOST /{p}/{tail*} wide"
        )),
        vec![
            "/",
            "  a",
            "    :int = num (GET)",
            "    :slug",
            "      ** = deep (GET,POST)",
            "    :any",
            "      b = mid (GET)",
            "  :any",
            "    ** = wide (POST)",
        ]
    );
}

#[test]
fn named_file_command_emits_one_exact_lf_terminated_output() {
    let io = fake_io(
        Ok(Vec::new()),
        vec![(
            String::from("routes.txt"),
            Ok(b"GET / home\nGET /a item".to_vec()),
        )],
    );
    assert_eq!(cmd_tree::run(&[String::from("routes.txt")], &io), Ok(0));
    assert_eq!(USAGE, "routeshadow tree FILE");
    assert_eq!(*io.reads.borrow(), vec![String::from("routes.txt")]);
    assert_eq!(
        *io.outputs.borrow(),
        vec![String::from("/ = home (GET)\n  a = item (GET)\n")]
    );
    assert!(io.errors.borrow().is_empty());
}

#[test]
fn stdin_command_reads_only_stdin_and_emits_exact_output() {
    let io = fake_io(
        Ok(b"GET / home\nGET /a item".to_vec()),
        vec![(String::from("unused.txt"), Ok(b"GET / wrong".to_vec()))],
    );
    assert_eq!(cmd_tree::run(&[String::from("-")], &io), Ok(0));
    assert_eq!(*io.reads.borrow(), vec![String::from("<stdin>")]);
    assert_eq!(
        *io.outputs.borrow(),
        vec![String::from("/ = home (GET)\n  a = item (GET)\n")]
    );
    assert!(io.errors.borrow().is_empty());
}

fn expect_validation_error(args: Vec<String>, expected: &str) {
    let io = fake_io(Ok(Vec::new()), Vec::new());
    assert_eq!(
        cmd_tree::run(&args, &io),
        Err(RsError(String::from(expected)))
    );
    assert!(io.reads.borrow().is_empty());
    assert!(io.outputs.borrow().is_empty());
    assert!(io.errors.borrow().is_empty());
}

#[test]
fn arity_errors_happen_without_reading_or_writing() {
    expect_validation_error(Vec::new(), "missing FILE");
    expect_validation_error(
        vec![String::from("a"), String::from("b")],
        "too many arguments",
    );
}

#[test]
fn options_are_checked_before_arity_and_first_unknown_option_wins() {
    expect_validation_error(vec![String::from("--help")], "unknown option \"--help\"");
    expect_validation_error(
        vec![
            String::from("a"),
            String::from("b"),
            String::from("--write"),
        ],
        "unknown option \"--write\"",
    );
    expect_validation_error(
        vec![String::from("--"), String::from("--help")],
        "unknown option \"--\"",
    );
    expect_validation_error(vec![String::from("--x=1")], "unknown option \"--x=1\"");
}

fn assert_no_command_output(io: &FakeIo) {
    assert!(io.outputs.borrow().is_empty());
    assert!(io.errors.borrow().is_empty());
}

#[test]
fn named_source_errors_propagate_without_output() {
    let empty = fake_io(
        Ok(Vec::new()),
        vec![(String::from("empty.txt"), Ok(b"# nothing".to_vec()))],
    );
    assert_eq!(
        cmd_tree::run(&[String::from("empty.txt")], &empty),
        Err(RsError(String::from("\"empty.txt\": no routes")))
    );
    assert_eq!(*empty.reads.borrow(), vec![String::from("empty.txt")]);
    assert_no_command_output(&empty);

    let malformed = fake_io(
        Ok(Vec::new()),
        vec![(
            String::from("bad.txt"),
            Ok(b"GET /a one\nPOST /b/ two".to_vec()),
        )],
    );
    assert_eq!(
        cmd_tree::run(&[String::from("bad.txt")], &malformed),
        Err(RsError(String::from(
            "\"bad.txt\": RS005 empty segment at 2:8"
        )))
    );
    assert_eq!(*malformed.reads.borrow(), vec![String::from("bad.txt")]);
    assert_no_command_output(&malformed);

    let unreadable = fake_io(
        Ok(Vec::new()),
        vec![(
            String::from("nowhere.txt"),
            Err(String::from("underlying read failure")),
        )],
    );
    assert_eq!(
        cmd_tree::run(&[String::from("nowhere.txt")], &unreadable),
        Err(RsError(String::from(
            "\"nowhere.txt\": cannot read the file"
        )))
    );
    assert_eq!(
        *unreadable.reads.borrow(),
        vec![String::from("nowhere.txt")]
    );
    assert_no_command_output(&unreadable);
}

#[test]
fn malformed_stdin_bytes_propagate_strict_utf8_error_without_output() {
    let io = fake_io(Ok(vec![65, 255, 10]), Vec::new());
    assert_eq!(
        cmd_tree::run(&[String::from("-")], &io),
        Err(RsError(String::from("<stdin>: input is not valid UTF-8")))
    );
    assert_eq!(*io.reads.borrow(), vec![String::from("<stdin>")]);
    assert_no_command_output(&io);
}
