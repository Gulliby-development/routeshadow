use routeshadow::{
    matcher::{Hit, find_all, match_route},
    model::Route,
    parser::parse,
};

fn parsed_routes(text: &str) -> Vec<Route> {
    let result = parse(text);
    assert!(result.is_ok());
    match result {
        Ok(routes) => routes,
        Err(_) => Vec::new(),
    }
}

fn match_text(route_text: &str, method: &str, path: &str) -> Option<Vec<(String, String)>> {
    let routes = parsed_routes(route_text);
    match routes.first() {
        Some(route) => match_route(route, method, path),
        None => None,
    }
}

fn match_row(input: [&str; 3]) -> Option<Vec<(String, String)>> {
    let [route_text, method, path] = input;
    match_text(route_text, method, path)
}

#[test]
fn root_method_and_fixed_segment_rows_match_the_contract() {
    assert_eq!(match_text("GET / home", "GET", "/"), Some(vec![]));
    assert_eq!(match_text("GET / home", "GET", "//"), None);
    assert_eq!(match_text("GET /a n", "GET", "/a"), Some(vec![]));
    assert_eq!(match_text("GET /a n", "POST", "/a"), None);
    assert_eq!(match_text("GET /a n", "get", "/a"), None);
    assert_eq!(
        match_text("* /health health", "OPTIONS", "/health"),
        Some(vec![])
    );
    assert_eq!(match_text("GET /a n", "GET", "/A"), None);
    assert_eq!(match_text("GET /a n", "GET", "/a/"), None);
    assert_eq!(match_text("GET /a n", "GET", "a"), None);
    assert_eq!(match_text("GET /a/b n", "GET", "/a"), None);
}

#[test]
fn integer_captures_accept_leading_zeroes_and_unbounded_digit_text() {
    assert_eq!(
        match_text("GET /{id:int} n", "GET", "/00042"),
        Some(vec![(String::from("id"), String::from("00042"))])
    );
    assert_eq!(match_text("GET /{id:int} n", "GET", "/4x"), None);
    assert_eq!(match_text("GET /{id:int} n", "GET", "/-1"), None);
    assert_eq!(
        match_text(
            "GET /{id:int} n",
            "GET",
            "/999999999999999999999999999999999999999999999999"
        ),
        Some(vec![(
            String::from("id"),
            String::from("999999999999999999999999999999999999999999999999")
        )])
    );
}

#[test]
fn slug_captures_accept_each_contract_example() {
    assert_eq!(
        match_text("GET /{x:slug} n", "GET", "/a"),
        Some(vec![(String::from("x"), String::from("a"))])
    );
    assert_eq!(
        match_text("GET /{x:slug} n", "GET", "/v2"),
        Some(vec![(String::from("x"), String::from("v2"))])
    );
    assert_eq!(
        match_text("GET /{x:slug} n", "GET", "/a-b"),
        Some(vec![(String::from("x"), String::from("a-b"))])
    );
    assert_eq!(
        match_text("GET /{x:slug} n", "GET", "/2024-10-21"),
        Some(vec![(String::from("x"), String::from("2024-10-21"))])
    ); // expected: [["x","2024-10-21"]]
}

#[test]
fn slug_captures_reject_uppercase_empty_groups_and_underscores() {
    assert_eq!(match_text("GET /{x:slug} n", "GET", "/A"), None);
    assert_eq!(match_text("GET /{x:slug} n", "GET", "/a--b"), None);
    assert_eq!(match_text("GET /{x:slug} n", "GET", "/-a"), None);
    assert_eq!(match_text("GET /{x:slug} n", "GET", "/a_b"), None);
}

#[test]
fn any_capture_and_literal_matching_preserve_raw_path_text() {
    assert_eq!(
        match_text("GET /{x:any} n", "GET", "/Q?x#y%2F"),
        Some(vec![(String::from("x"), String::from("Q?x#y%2F"))])
    ); // expected: [["x","Q?x#y%2F"]]
    assert_eq!(match_text("GET /{x} n", "GET", "/"), None);
    assert_eq!(match_text("GET /a n", "GET", "/%61"), None);
    assert_eq!(match_text("GET /a n", "GET", "/a?q"), None);
    assert_eq!(match_text("GET /./.. n", "GET", "/./.."), Some(vec![]));
}

#[test]
fn parameter_values_follow_pattern_order() {
    assert_eq!(
        match_text("GET /{z}/{a:int} n", "GET", "/Q/01"),
        Some(vec![
            (String::from("z"), String::from("Q")),
            (String::from("a"), String::from("01"))
        ])
    );
}

#[test]
fn acceptance_rows_cover_all_literal_matcher_boundaries() {
    assert_eq!(match_row(["GET /a n", "GET", "/A"]), None);
    assert_eq!(match_row(["GET /{id:int} n", "GET", "/4x"]), None);
    assert_eq!(match_row(["GET /{id:int} n", "GET", "/-1"]), None);
    assert_eq!(
        match_row(["GET /{x:slug} n", "GET", "/2024-10-21"]),
        Some(vec![(String::from("x"), String::from("2024-10-21"))])
    );
    assert_eq!(match_row(["GET /{x:slug} n", "GET", "/A"]), None);
    assert_eq!(match_row(["GET /{x:slug} n", "GET", "/a--b"]), None);
    assert_eq!(match_row(["GET /{x:slug} n", "GET", "/-a"]), None);
    assert_eq!(match_row(["GET /{x:slug} n", "GET", "/a_b"]), None);
    assert_eq!(
        match_row(["GET /{x:any} n", "GET", "/Q?x#y%2F"]),
        Some(vec![(String::from("x"), String::from("Q?x#y%2F"))])
    );
    assert_eq!(
        match_row(["GET /{z}/{a:int} n", "GET", "/Q/01"]),
        Some(vec![
            (String::from("z"), String::from("Q")),
            (String::from("a"), String::from("01"))
        ])
    );
    assert_eq!(match_row(["GET /a n", "GET", "/%61"]), None);
    assert_eq!(
        match_row(["GET /files/{path*} n", "GET", "/files/a/b.txt"]),
        Some(vec![(String::from("path"), String::from("a/b.txt"))])
    );
    assert_eq!(
        match_row(["GET /files/{path*} n", "GET", "/files/a"]),
        Some(vec![(String::from("path"), String::from("a"))])
    );
    assert_eq!(match_row(["GET /files/{path*} n", "GET", "/files"]), None);
    assert_eq!(match_row(["GET /files/{path*} n", "GET", "/files/"]), None);
    assert_eq!(
        match_row(["GET /files/{path*} n", "GET", "/files/a//b"]),
        None
    );
    assert_eq!(
        match_row(["GET /files/{path*} n", "GET", "/files/a/"]),
        None
    );
    assert_eq!(
        match_row(["GET /{z:slug}/{a*} n", "GET", "/q/b/c"]),
        Some(vec![
            (String::from("z"), String::from("q")),
            (String::from("a"), String::from("b/c"))
        ])
    );
    assert_eq!(match_row(["GET /{x}/{y} n", "GET", "//b"]), None);
}

#[test]
fn rest_captures_one_or_more_nonempty_segments_joined_by_slashes() {
    assert_eq!(
        match_text("GET /files/{path*} n", "GET", "/files/a/b.txt"),
        Some(vec![(String::from("path"), String::from("a/b.txt"))])
    );
    assert_eq!(
        match_text("GET /files/{path*} n", "GET", "/files/a"),
        Some(vec![(String::from("path"), String::from("a"))])
    );
    assert_eq!(
        match_text("GET /{rest*} n", "GET", "/a/b"),
        Some(vec![(String::from("rest"), String::from("a/b"))])
    );
    assert_eq!(
        match_text("GET /{z:slug}/{a*} n", "GET", "/q/b/c"),
        Some(vec![
            (String::from("z"), String::from("q")),
            (String::from("a"), String::from("b/c"))
        ])
    );
}

#[test]
fn rest_rejects_missing_or_empty_remaining_segments() {
    assert_eq!(match_text("GET /files/{path*} n", "GET", "/files"), None);
    assert_eq!(match_text("GET /files/{path*} n", "GET", "/files/"), None);
    assert_eq!(
        match_text("GET /files/{path*} n", "GET", "/files/a//b"),
        None
    );
    assert_eq!(match_text("GET /files/{path*} n", "GET", "/files/a/"), None);
    assert_eq!(match_text("GET /{rest*} n", "GET", "/"), None);
}

#[test]
fn rest_matching_rejects_empty_fixed_segments_before_the_rest() {
    assert_eq!(match_text("GET /{x}/{y} n", "GET", "//b"), None);
}

#[test]
fn find_all_returns_every_hit_in_route_order_with_zero_based_indices() {
    let routes = parsed_routes("POST /a skip\nGET /{x} first\nGET /a second");
    assert_eq!(
        find_all(&routes, "GET", "/a"),
        vec![
            Hit {
                index: 1,
                values: vec![(String::from("x"), String::from("a"))]
            },
            Hit {
                index: 2,
                values: vec![]
            }
        ]
    );
}

#[test]
fn find_all_returns_no_hits_for_method_mismatch_or_empty_routes() {
    let routes = parsed_routes("GET /a n");
    assert_eq!(find_all(&routes, "DELETE", "/a"), vec![]);
    let no_routes = parsed_routes("");
    assert_eq!(find_all(&no_routes, "GET", "/"), vec![]);
}

#[test]
fn hit_fields_and_standard_value_traits_are_public() {
    let hit = Hit {
        index: 7,
        values: vec![(String::from("id"), String::from("42"))],
    };
    assert_eq!(hit.index, 7);
    assert_eq!(hit.values, vec![(String::from("id"), String::from("42"))]);
    assert_eq!(hit.clone(), hit);
    assert_eq!(
        format!("{hit:?}"),
        "Hit { index: 7, values: [(\"id\", \"42\")] }"
    );
    assert_ne!(
        hit,
        Hit {
            index: 8,
            values: vec![(String::from("id"), String::from("42"))]
        }
    );
}
