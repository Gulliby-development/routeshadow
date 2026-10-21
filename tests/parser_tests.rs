use routeshadow::{parser::parse, problem::Problem};

fn assert_problem(
    input: &str,
    code: &'static str,
    text: &str,
    line: usize,
    col: usize,
    rendered: &str,
) {
    let problem = match parse(input) {
        Err(problem) => problem,
        Ok(routes) => panic!("expected a parser problem, got {routes:?}"),
    };

    assert_eq!(problem.code, code);
    assert_eq!(problem.text, text);
    assert_eq!(problem.line, line);
    assert_eq!(problem.col, col);
    assert_eq!(problem.render(), rendered);
    assert_eq!(problem.clone(), problem);
    let _debug = format!("{problem:?}");
    assert_eq_trait::<Problem>();
}

fn assert_eq_trait<T: Eq>() {}

#[test]
fn non_graphic_characters_are_reported_at_utf16_columns() {
    assert_problem(
        "GET\t/a n",
        "RS001",
        "unexpected character \"?\"",
        1,
        4,
        "RS001 unexpected character \"?\" at 1:4",
    );
    assert_problem(
        "GET /a n😀",
        "RS001",
        "unexpected character \"?\"",
        1,
        9,
        "RS001 unexpected character \"?\" at 1:9",
    );
    assert_problem(
        "GET aé n",
        "RS001",
        "unexpected character \"?\"",
        1,
        6,
        "RS001 unexpected character \"?\" at 1:6",
    );
    assert_problem(
        "GET /a\rb n",
        "RS001",
        "unexpected character \"?\"",
        1,
        7,
        "RS001 unexpected character \"?\" at 1:7",
    );
    assert_problem(
        "GET /a n\r\r\n",
        "RS001",
        "unexpected character \"?\"",
        1,
        9,
        "RS001 unexpected character \"?\" at 1:9",
    );
}

#[test]
fn invalid_method_parts_are_checked_before_later_tokens() {
    assert_problem(
        "get /a n",
        "RS002",
        "invalid method \"get\"",
        1,
        1,
        "RS002 invalid method \"get\" at 1:1",
    );
    assert_problem(
        "GET,,POST /a n",
        "RS002",
        "invalid method \"\"",
        1,
        5,
        "RS002 invalid method \"\" at 1:5",
    );
    assert_problem(
        "GET,* /a n",
        "RS002",
        "invalid method \"*\"",
        1,
        5,
        "RS002 invalid method \"*\" at 1:5",
    );
    assert_problem(
        "GET, /a n",
        "RS002",
        "invalid method \"\"",
        1,
        5,
        "RS002 invalid method \"\" at 1:5",
    );
    assert_problem(
        "get /a né",
        "RS002",
        "invalid method \"get\"",
        1,
        1,
        "RS002 invalid method \"get\" at 1:1",
    );
}

#[test]
fn repeated_methods_report_the_second_part() {
    assert_problem(
        "GET,POST,GET /a n",
        "RS003",
        "repeated method \"GET\"",
        1,
        10,
        "RS003 repeated method \"GET\" at 1:10",
    );
}

#[test]
fn patterns_must_begin_with_a_slash() {
    assert_problem(
        "GET a n",
        "RS004",
        "pattern must start with \"/\"",
        1,
        5,
        "RS004 pattern must start with \"/\" at 1:5",
    );
}

#[test]
fn empty_segments_report_the_opening_slash() {
    assert_problem(
        "GET /a/ n",
        "RS005",
        "empty segment",
        1,
        7,
        "RS005 empty segment at 1:7",
    );
    assert_problem(
        "GET // n",
        "RS005",
        "empty segment",
        1,
        6,
        "RS005 empty segment at 1:6",
    );
}

#[test]
fn malformed_segments_braces_and_parameter_names_are_rejected() {
    assert_problem(
        "GET /a*b n",
        "RS006",
        "invalid segment \"a*b\"",
        1,
        6,
        "RS006 invalid segment \"a*b\" at 1:6",
    );
    assert_problem(
        "GET /{id:int*} n",
        "RS006",
        "invalid segment \"{id:int*}\"",
        1,
        6,
        "RS006 invalid segment \"{id:int*}\" at 1:6",
    );
    assert_problem(
        "GET /{Bad:nope} n",
        "RS006",
        "invalid segment \"{Bad:nope}\"",
        1,
        6,
        "RS006 invalid segment \"{Bad:nope}\" at 1:6",
    );
    assert_problem(
        "GET /{{a}} n",
        "RS006",
        "invalid segment \"{{a}}\"",
        1,
        6,
        "RS006 invalid segment \"{{a}}\" at 1:6",
    );
}

#[test]
fn unknown_types_report_the_type_start_before_later_segments() {
    assert_problem(
        "GET /{id:} n",
        "RS007",
        "unknown type \"\"",
        1,
        10,
        "RS007 unknown type \"\" at 1:10",
    );
    assert_problem(
        "GET /{a:b:c} n",
        "RS007",
        "unknown type \"b:c\"",
        1,
        9,
        "RS007 unknown type \"b:c\" at 1:9",
    );
    assert_problem(
        "GET /{a:nope}// n",
        "RS007",
        "unknown type \"nope\"",
        1,
        9,
        "RS007 unknown type \"nope\" at 1:9",
    );
}

#[test]
fn rest_placement_precedes_duplicate_capture_validation() {
    assert_problem(
        "GET /{p*}/x n",
        "RS008",
        "rest segment must be last",
        1,
        6,
        "RS008 rest segment must be last at 1:6",
    );
    assert_problem(
        "GET /{a}/{a*}/x n",
        "RS008",
        "rest segment must be last",
        1,
        10,
        "RS008 rest segment must be last at 1:10",
    );
}

#[test]
fn duplicate_captures_are_reported_at_the_repeated_name() {
    assert_problem(
        "GET /{a}/{a:int} n",
        "RS009",
        "duplicate parameter \"a\"",
        1,
        11,
        "RS009 duplicate parameter \"a\" at 1:11",
    );
}

#[test]
fn invalid_route_names_are_reported_before_excess_tokens() {
    assert_problem(
        "GET /a 1x",
        "RS010",
        "invalid name \"1x\"",
        1,
        8,
        "RS010 invalid name \"1x\" at 1:8",
    );
    assert_problem(
        "GET /a 1x extra",
        "RS010",
        "invalid name \"1x\"",
        1,
        8,
        "RS010 invalid name \"1x\" at 1:8",
    );
}

#[test]
fn duplicate_route_names_are_reported_at_the_later_name() {
    assert_problem(
        "GET /a n\nPOST /b n",
        "RS011",
        "duplicate name \"n\"",
        2,
        9,
        "RS011 duplicate name \"n\" at 2:9",
    );
    assert_problem(
        "GET /a n\nPOST /b n extra",
        "RS011",
        "duplicate name \"n\"",
        2,
        9,
        "RS011 duplicate name \"n\" at 2:9",
    );
}

#[test]
fn missing_tokens_use_the_full_line_end_column() {
    assert_problem(
        "GET",
        "RS012",
        "missing pattern",
        1,
        4,
        "RS012 missing pattern at 1:4",
    );
    assert_problem(
        "GET /a  ",
        "RS012",
        "missing name",
        1,
        9,
        "RS012 missing name at 1:9",
    );
}

#[test]
fn excess_tokens_are_reported_at_the_fourth_token() {
    assert_problem(
        "GET /a n extra",
        "RS013",
        "unexpected token \"extra\"",
        1,
        10,
        "RS013 unexpected token \"extra\" at 1:10",
    );
}

#[test]
fn physical_lines_and_first_error_precedence_are_preserved() {
    assert_problem(
        "# routes\n\nGET /a n\nGET b m",
        "RS004",
        "pattern must start with \"/\"",
        4,
        5,
        "RS004 pattern must start with \"/\" at 4:5",
    );
    assert_problem(
        "GET /a n\nPOST b n extra",
        "RS004",
        "pattern must start with \"/\"",
        2,
        6,
        "RS004 pattern must start with \"/\" at 2:6",
    );
}
