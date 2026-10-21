use std::{cell::RefCell, collections::BTreeMap};

use routeshadow::{
    cmd_check, cmd_list,
    error::RsError,
    io::Io,
    parser::parse,
    stats::{self, Stats},
};

struct FakeIo {
    files: RefCell<BTreeMap<String, Vec<u8>>>,
    stdin: RefCell<Vec<u8>>,
    reads: RefCell<Vec<String>>,
    stdout: RefCell<Vec<String>>,
    stderr: RefCell<Vec<String>>,
}

impl Io for FakeIo {
    fn read_stdin(&self) -> Result<Vec<u8>, String> {
        self.reads.borrow_mut().push(String::from("stdin"));
        Ok(self.stdin.borrow().clone())
    }

    fn read_file(&self, path: &str) -> Result<Vec<u8>, String> {
        self.reads.borrow_mut().push(String::from(path));
        match self.files.borrow().get(path) {
            Some(bytes) => Ok(bytes.clone()),
            None => Err(String::from("missing file")),
        }
    }

    fn out(&self, text: &str) {
        self.stdout.borrow_mut().push(String::from(text));
    }

    fn err(&self, text: &str) {
        self.stderr.borrow_mut().push(String::from(text));
    }
}

fn fake_io(file: Option<(&str, &[u8])>, stdin: &[u8]) -> FakeIo {
    let mut files = BTreeMap::new();
    if let Some((path, bytes)) = file {
        files.insert(String::from(path), bytes.to_vec());
    }
    FakeIo {
        files: RefCell::new(files),
        stdin: RefCell::new(stdin.to_vec()),
        reads: RefCell::new(Vec::new()),
        stdout: RefCell::new(Vec::new()),
        stderr: RefCell::new(Vec::new()),
    }
}

fn args(values: &[&str]) -> Vec<String> {
    values.iter().map(|value| String::from(*value)).collect()
}

fn assert_no_output(io: &FakeIo) {
    assert!(io.stdout.borrow().is_empty());
    assert!(io.stderr.borrow().is_empty());
}

fn assert_one_output(io: &FakeIo, expected: &str) {
    assert_eq!(io.stdout.borrow().as_slice(), &[String::from(expected)]);
    assert!(io.stderr.borrow().is_empty());
}

fn assert_stats(text: &str, expected: Stats) {
    let parsed = parse(text);
    assert!(parsed.is_ok());
    if let Ok(routes) = parsed {
        let actual = stats::stats(&routes);
        assert_eq!(actual, expected);
        assert_eq!(actual.clone(), expected);
        assert_eq!(format!("{actual:?}"), format!("{expected:?}"));
    }
}

fn app_routes() -> &'static [u8] {
    b"# a small web app\nGET / home\nGET,POST /users users\nGET /users/me users_me\nGET /users/{id:int} users_show\nGET /users/{name} users_by_name\nGET /files/{path*} files_get\nGET /files/readme.txt files_readme\n* /health health\nGET /health health_get\nGET /{kind}/latest latest\n"
}

#[test]
fn stats_counts_empty_input_as_all_zeroes() {
    assert_stats(
        "",
        Stats {
            routes: 0,
            fixed: 0,
            dynamic: 0,
            rest: 0,
            any_method: 0,
            deepest: 0,
        },
    );
}

#[test]
fn stats_counts_the_root_as_fixed_with_zero_depth() {
    assert_stats(
        "GET / home\n",
        Stats {
            routes: 1,
            fixed: 1,
            dynamic: 0,
            rest: 0,
            any_method: 0,
            deepest: 0,
        },
    );
}

#[test]
fn stats_counts_all_methods_rest_routes_and_maximum_depth() {
    assert_stats(
        "* /health health\nOPTIONS,HEAD,DELETE,PATCH,PUT,POST,GET /all all\nGET /a/{id:int}/{tail*} deep\nGET /{name} dynamic\n",
        Stats {
            routes: 4,
            fixed: 2,
            dynamic: 2,
            rest: 1,
            any_method: 2,
            deepest: 3,
        },
    );
}

#[test]
fn option_and_arity_errors_precede_reads_and_writes() {
    assert_eq!(cmd_check::USAGE, "routeshadow check FILE");
    assert_eq!(cmd_list::USAGE, "routeshadow list FILE [--by-name]");

    let io = fake_io(None, b"");
    assert_eq!(
        cmd_check::run(&args(&[]), &io),
        Err(RsError(String::from("missing FILE")))
    );
    assert_eq!(
        cmd_check::run(&args(&["a", "b"]), &io),
        Err(RsError(String::from("too many arguments")))
    );
    assert_eq!(
        cmd_check::run(&args(&["a", "b", "--help"]), &io),
        Err(RsError(String::from("unknown option \"--help\"")))
    );
    assert_eq!(
        cmd_check::run(&args(&["--"]), &io),
        Err(RsError(String::from("unknown option \"--\"")))
    );
    assert_eq!(
        cmd_list::run(&args(&["app.txt", "--write"]), &io),
        Err(RsError(String::from("unknown option \"--write\"")))
    );
    assert_eq!(
        cmd_list::run(&args(&["--by-name"]), &io),
        Err(RsError(String::from("missing FILE")))
    );
    assert_eq!(
        cmd_list::run(&args(&["--by-name=1"]), &io),
        Err(RsError(String::from("unknown option \"--by-name=1\"")))
    );
    assert!(io.reads.borrow().is_empty());
    assert_no_output(&io);
}

#[test]
fn check_prints_the_exact_app_summary_once() {
    let io = fake_io(Some(("app.txt", app_routes())), b"");
    assert_eq!(cmd_check::run(&args(&["app.txt"]), &io), Ok(0));
    assert_eq!(io.reads.borrow().as_slice(), &[String::from("app.txt")]);
    assert_one_output(
        &io,
        "ok, 10 routes\nstatic      6\ndynamic     4\nrest        1\nany-method  1\ndeepest     2\n",
    );
}

#[test]
fn check_uses_singular_route_wording_for_a_root_route() {
    let io = fake_io(Some(("root.txt", b"GET / home\n")), b"");
    assert_eq!(cmd_check::run(&args(&["root.txt"]), &io), Ok(0));
    assert_one_output(
        &io,
        "ok, 1 route\nstatic      1\ndynamic     0\nrest        0\nany-method  0\ndeepest     0\n",
    );
}

#[test]
fn check_reports_only_the_first_problem_and_list_adds_file_context() {
    let bad_routes = b"GET /a one\nPOST /b/ two\nget /x later\n";

    let check_io = fake_io(Some(("bad.txt", bad_routes)), b"");
    assert_eq!(cmd_check::run(&args(&["bad.txt"]), &check_io), Ok(1));
    assert_one_output(&check_io, "RS005 empty segment at 2:8\n1 problem\n");
    assert_eq!(
        check_io.reads.borrow().as_slice(),
        &[String::from("bad.txt")]
    );

    let list_io = fake_io(Some(("bad.txt", bad_routes)), b"");
    assert_eq!(
        cmd_list::run(&args(&["bad.txt"]), &list_io),
        Err(RsError(String::from(
            "\"bad.txt\": RS005 empty segment at 2:8"
        )))
    );
    assert_eq!(
        list_io.reads.borrow().as_slice(),
        &[String::from("bad.txt")]
    );
    assert_no_output(&list_io);
}

#[test]
fn check_and_list_propagate_read_empty_and_invalid_stdin_errors() {
    let missing_io = fake_io(None, b"");
    assert_eq!(
        cmd_check::run(&args(&["nowhere.txt"]), &missing_io),
        Err(RsError(String::from(
            "\"nowhere.txt\": cannot read the file"
        )))
    );
    assert_no_output(&missing_io);
    assert_eq!(
        cmd_list::run(&args(&["nowhere.txt"]), &missing_io),
        Err(RsError(String::from(
            "\"nowhere.txt\": cannot read the file"
        )))
    );
    assert_no_output(&missing_io);
    assert_eq!(
        missing_io.reads.borrow().as_slice(),
        &[String::from("nowhere.txt"), String::from("nowhere.txt")]
    );

    let empty_io = fake_io(Some(("empty.txt", b"# nothing\n")), b"");
    assert_eq!(
        cmd_check::run(&args(&["empty.txt"]), &empty_io),
        Err(RsError(String::from("\"empty.txt\": no routes")))
    );
    assert_no_output(&empty_io);
    assert_eq!(
        cmd_list::run(&args(&["empty.txt"]), &empty_io),
        Err(RsError(String::from("\"empty.txt\": no routes")))
    );
    assert_no_output(&empty_io);

    let invalid_stdin_io = fake_io(None, &[65, 255, 10]);
    assert_eq!(
        cmd_check::run(&args(&["-"]), &invalid_stdin_io),
        Err(RsError(String::from("<stdin>: input is not valid UTF-8")))
    );
    assert_no_output(&invalid_stdin_io);
    assert_eq!(
        cmd_list::run(&args(&["-"]), &invalid_stdin_io),
        Err(RsError(String::from("<stdin>: input is not valid UTF-8")))
    );
    assert_no_output(&invalid_stdin_io);
    assert_eq!(
        invalid_stdin_io.reads.borrow().as_slice(),
        &[String::from("stdin"), String::from("stdin")]
    );
}

#[test]
fn check_and_list_accept_leading_bom_and_crlf_stdin() {
    let check_io = fake_io(None, b"\xef\xbb\xbfGET / home\r\n");
    assert_eq!(cmd_check::run(&args(&["-"]), &check_io), Ok(0));
    assert_eq!(check_io.reads.borrow().as_slice(), &[String::from("stdin")]);
    assert_one_output(
        &check_io,
        "ok, 1 route\nstatic      1\ndynamic     0\nrest        0\nany-method  0\ndeepest     0\n",
    );

    let list_io = fake_io(None, b"\xef\xbb\xbfGET / home\r\n");
    assert_eq!(cmd_list::run(&args(&["-"]), &list_io), Ok(0));
    assert_eq!(list_io.reads.borrow().as_slice(), &[String::from("stdin")]);
    assert_one_output(
        &list_io,
        "LINE  METHODS  PATTERN  NAME\n1     GET      /        home\n",
    );
}

#[test]
fn list_matches_the_app_example_in_file_order() {
    let io = fake_io(Some(("app.txt", app_routes())), b"");
    assert_eq!(cmd_list::run(&args(&["app.txt"]), &io), Ok(0));
    assert_eq!(io.reads.borrow().as_slice(), &[String::from("app.txt")]);
    assert_one_output(
        &io,
        "LINE  METHODS   PATTERN            NAME\n2     GET       /                  home\n3     GET,POST  /users             users\n4     GET       /users/me          users_me\n5     GET       /users/{id:int}    users_show\n6     GET       /users/{name}      users_by_name\n7     GET       /files/{path*}     files_get\n8     GET       /files/readme.txt  files_readme\n9     *         /health            health\n10    GET       /health            health_get\n11    GET       /{kind}/latest     latest\n",
    );
}

#[test]
fn list_sorts_app_rows_by_name_with_flags_anywhere_and_repeated() {
    let sorted = "LINE  METHODS   PATTERN            NAME\n7     GET       /files/{path*}     files_get\n8     GET       /files/readme.txt  files_readme\n9     *         /health            health\n10    GET       /health            health_get\n2     GET       /                  home\n11    GET       /{kind}/latest     latest\n3     GET,POST  /users             users\n6     GET       /users/{name}      users_by_name\n4     GET       /users/me          users_me\n5     GET       /users/{id:int}    users_show\n";

    let last_flag_io = fake_io(Some(("app.txt", app_routes())), b"");
    assert_eq!(
        cmd_list::run(&args(&["app.txt", "--by-name"]), &last_flag_io),
        Ok(0)
    );
    assert_one_output(&last_flag_io, sorted);

    let repeated_flag_io = fake_io(Some(("app.txt", app_routes())), b"");
    assert_eq!(
        cmd_list::run(
            &args(&["--by-name", "app.txt", "--by-name"]),
            &repeated_flag_io
        ),
        Ok(0)
    );
    assert_one_output(&repeated_flag_io, sorted);
}

#[test]
fn list_sorts_names_bytewise_and_preserves_patterns_and_method_order() {
    let routes = b"# routes\nPOST,GET /{x:any} z\nGET / A\nGET /a a\n";
    let io = fake_io(Some(("mixed.txt", routes)), b"");
    assert_eq!(cmd_list::run(&args(&["mixed.txt"]), &io), Ok(0));
    assert_one_output(
        &io,
        "LINE  METHODS   PATTERN   NAME\n2     GET,POST  /{x:any}  z\n3     GET       /         A\n4     GET       /a        a\n",
    );

    let sorted_io = fake_io(Some(("mixed.txt", routes)), b"");
    assert_eq!(
        cmd_list::run(&args(&["--by-name", "mixed.txt"]), &sorted_io),
        Ok(0)
    );
    assert_one_output(
        &sorted_io,
        "LINE  METHODS   PATTERN   NAME\n3     GET       /         A\n4     GET       /a        a\n2     GET,POST  /{x:any}  z\n",
    );
}
