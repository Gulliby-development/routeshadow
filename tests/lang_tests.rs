use routeshadow::{
    lang::{inside, witness},
    model::{Seg, Ty},
    parser::parse,
};

fn lit(text: &str) -> Seg {
    Seg::Lit(text.to_owned())
}

fn param(name: &str, ty: Ty) -> Seg {
    Seg::Param {
        name: name.to_owned(),
        ty,
    }
}

fn rest(name: &str) -> Seg {
    Seg::Rest {
        name: name.to_owned(),
    }
}

fn parsed_segments(pattern: &str) -> Vec<Seg> {
    let source = format!("GET {pattern} route");
    let routes = match parse(&source) {
        Ok(routes) => routes,
        Err(problem) => panic!("test pattern did not parse: {}", problem.render()),
    };
    match routes.into_iter().next() {
        Some(route) => route.segs,
        None => panic!("test pattern did not produce a route"),
    }
}

fn assert_inside(y: Vec<Seg>, x: Vec<Seg>, expected: bool) {
    assert_eq!(inside(&y, &x), expected);
}

fn assert_witness(x: Vec<Seg>, y: Vec<Seg>, expected: Option<Vec<String>>) {
    assert_eq!(witness(&x, &y), expected);
    assert_eq!(witness(&y, &x), expected);
}

#[test]
fn inside_accepts_root_only_in_root_and_fixed_patterns_of_equal_length() {
    assert_inside(vec![], vec![], true);
    assert_inside(vec![], vec![rest("r")], false);
    assert_inside(vec![lit("a")], vec![lit("a"), lit("b")], false);
    assert_inside(vec![lit("a")], vec![lit("b")], false);
}

#[test]
fn inside_fixed_patterns_use_segment_subset_and_ignore_parameter_names() {
    assert_inside(
        vec![lit("a"), param("n", Ty::Int)],
        vec![lit("a"), param("id", Ty::Any)],
        true,
    );
    assert_inside(
        vec![lit("a"), param("id", Ty::Any)],
        vec![lit("a"), param("n", Ty::Int)],
        false,
    );
    assert_inside(
        vec![lit("a"), param("id", Ty::Any)],
        vec![lit("a"), param("n", Ty::Any)],
        true,
    );
}

#[test]
fn fixed_pattern_is_inside_rest_only_after_a_nonempty_tail() {
    assert_inside(vec![lit("a")], vec![lit("a"), rest("r")], false);
    assert_inside(parsed_segments("/a/b"), parsed_segments("/a/{r*}"), true);
    assert_inside(parsed_segments("/b/x"), parsed_segments("/a/{r*}"), false);
}

#[test]
fn rest_pattern_containment_checks_its_fixed_prefix() {
    assert_inside(vec![lit("a"), rest("s")], vec![lit("a"), rest("r")], true);
    assert_inside(parsed_segments("/a/{r*}"), parsed_segments("/a/b"), false);
    assert_inside(vec![rest("s")], vec![lit("a"), rest("r")], false);
    assert_inside(
        parsed_segments("/a/b/{s*}"),
        parsed_segments("/a/{r*}"),
        true,
    );
}

#[test]
fn a_rest_pattern_never_contains_a_fixed_pattern_of_its_prefix_length() {
    assert_inside(vec![lit("a")], vec![lit("a"), rest("r")], false);
    assert_inside(vec![lit("a"), lit("b")], vec![lit("a"), rest("r")], true);
}

#[test]
fn witness_of_root_and_incompatible_fixed_lengths_is_canonical() {
    assert_witness(vec![], vec![], Some(vec![]));
    assert_witness(vec![], vec![rest("r")], None);
    assert_witness(parsed_segments("/a"), parsed_segments("/a/b"), None);
    assert_witness(vec![lit("a")], vec![lit("b")], None);
}

#[test]
fn fixed_pattern_witnesses_use_the_smallest_common_segment_samples() {
    assert_witness(
        vec![param("i", Ty::Int)],
        vec![param("s", Ty::Slug)],
        Some(vec![String::from("1")]),
    );
    assert_witness(
        vec![param("s", Ty::Slug)],
        vec![param("a", Ty::Any)],
        Some(vec![String::from("a")]),
    );
    assert_witness(
        vec![param("a", Ty::Any)],
        vec![param("b", Ty::Any)],
        Some(vec![String::from("x")]),
    );
}

#[test]
fn one_rest_cannot_intersect_a_fixed_pattern_without_a_tail_segment() {
    assert_witness(vec![lit("a"), rest("r")], vec![lit("a")], None);
    assert_witness(
        vec![lit("a"), rest("r")],
        vec![lit("a"), param("i", Ty::Int), lit("z")],
        Some(vec![
            String::from("a"),
            String::from("1"),
            String::from("z"),
        ]),
    );
}

#[test]
fn rest_witnesses_sample_the_other_pattern_without_empty_tail() {
    assert_witness(
        vec![lit("a"), rest("r")],
        vec![lit("a"), rest("s")],
        Some(vec![String::from("a"), String::from("x")]),
    );
    assert_witness(
        vec![rest("r")],
        vec![lit("a"), param("i", Ty::Int), rest("tail")],
        Some(vec![
            String::from("a"),
            String::from("1"),
            String::from("x"),
        ]),
    );
}

#[test]
fn two_rest_witnesses_have_the_shortest_nonempty_shared_tail() {
    assert_witness(
        vec![lit("a"), rest("r")],
        vec![lit("a"), rest("s")],
        Some(vec![String::from("a"), String::from("x")]),
    );
}

#[test]
fn witness_returns_none_when_shared_prefix_languages_are_disjoint() {
    assert_witness(vec![lit("a"), rest("r")], vec![lit("b"), rest("s")], None);
    assert_witness(vec![param("s", Ty::Slug)], vec![lit("A")], None);
}
