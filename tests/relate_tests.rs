use routeshadow::{
    model::Route,
    parser::parse,
    relate::{Overlap, Shadow, contains, overlaps, shadows},
};

fn routes(text: &str) -> Vec<Route> {
    match parse(text) {
        Ok(routes) => routes,
        Err(problem) => panic!("test route text did not parse: {}", problem.render()),
    }
}

#[test]
fn contains_uses_path_languages_and_ignores_parameter_names() {
    let same_languages = routes("GET /a/{id} x\nGET /a/{n} y");
    assert!(contains(&same_languages[0], &same_languages[1]));
    assert!(contains(&same_languages[1], &same_languages[0]));

    let narrower = routes("GET /a/{id:int} x\nGET /a/{n} y");
    assert!(contains(&narrower[1], &narrower[0]));
    assert!(!contains(&narrower[0], &narrower[1]));
}

#[test]
fn contains_requires_method_subset_and_handles_all_methods_and_rest_boundaries() {
    let routes =
        routes("GET,POST /a x\nGET /a y\nPOST /a z\n* /{r*} all\nOPTIONS /a/b options\nGET / root");
    assert!(contains(&routes[0], &routes[1]));
    assert!(!contains(&routes[1], &routes[0]));
    assert!(!contains(&routes[2], &routes[1]));
    assert!(contains(&routes[3], &routes[4]));
    assert!(!contains(&routes[3], &routes[5]));
}

#[test]
fn all_method_rest_contains_the_exact_options_tail_case() {
    let parsed = routes("* /{r*} x\nOPTIONS /a/b y");
    assert!(contains(&parsed[0], &parsed[1]));
}

#[test]
fn shadows_choose_the_earliest_individual_containing_route() {
    let parsed = routes("GET /a/{x} first\nGET /a/{y} second\nGET /a/1 third");
    assert_eq!(
        shadows(&parsed),
        vec![Shadow { route: 1, by: 0 }, Shadow { route: 2, by: 0 },]
    );
}

#[test]
fn shadows_do_not_union_methods_or_report_specific_routes_shadowing_general_ones() {
    let separate_methods = routes("GET /a first\nPOST /a second\nGET,POST /a third");
    assert_eq!(shadows(&separate_methods), Vec::<Shadow>::new());

    let specific_first = routes("GET /users/me specific\nGET /users/{name} general");
    assert_eq!(shadows(&specific_first), Vec::<Shadow>::new());
}

#[test]
fn shadows_use_slice_positions_even_when_physical_lines_have_gaps() {
    let parsed = routes("# note\nGET /a/{r*} broad\n\nGET /a/b narrow");
    assert_eq!(parsed[1].line, 4);
    assert_eq!(shadows(&parsed), vec![Shadow { route: 1, by: 0 }]);
}

#[test]
fn overlaps_exclude_containment_disjoint_methods_and_disjoint_paths() {
    let specific_first = routes("GET /users/me specific\nGET /users/{name} general");
    assert_eq!(overlaps(&specific_first), Vec::<Overlap>::new());

    let method_subset = routes("GET /a one\nGET,POST /a two");
    assert_eq!(overlaps(&method_subset), Vec::<Overlap>::new());

    let disjoint_methods = routes("GET /a/{x} one\nPOST /{y}/b two");
    assert_eq!(overlaps(&disjoint_methods), Vec::<Overlap>::new());

    let disjoint_paths = routes("GET /a one\nGET /b two");
    assert_eq!(overlaps(&disjoint_paths), Vec::<Overlap>::new());
}

#[test]
fn overlaps_use_the_canonical_method_and_path_for_crossing_routes() {
    let parsed = routes("POST,GET /a/{x} one\nGET,POST /{y}/b two\nGET /{z}/c three");
    assert_eq!(
        overlaps(&parsed),
        vec![
            Overlap {
                first: 0,
                second: 1,
                method: String::from("GET"),
                path: String::from("/a/b"),
            },
            Overlap {
                first: 0,
                second: 2,
                method: String::from("GET"),
                path: String::from("/a/c"),
            },
        ]
    );
}

#[test]
fn overlaps_include_root_intersections_and_pick_the_first_common_method() {
    let parsed = routes("GET,POST / first\nPOST,PUT / second");
    assert_eq!(
        overlaps(&parsed),
        vec![Overlap {
            first: 0,
            second: 1,
            method: String::from("POST"),
            path: String::from("/"),
        }]
    );
}

#[test]
fn overlaps_order_each_later_route_by_earlier_route_position() {
    let parsed =
        routes("GET /{a}/{b:int} first\nGET /{c:int}/{d} second\nGET /{e:slug}/{f:slug} third");
    assert_eq!(
        overlaps(&parsed),
        vec![
            Overlap {
                first: 0,
                second: 1,
                method: String::from("GET"),
                path: String::from("/1/1"),
            },
            Overlap {
                first: 0,
                second: 2,
                method: String::from("GET"),
                path: String::from("/a/1"),
            },
            Overlap {
                first: 1,
                second: 2,
                method: String::from("GET"),
                path: String::from("/1/a"),
            },
        ]
    );
}

#[test]
fn empty_and_single_route_slices_have_no_relations() {
    assert_eq!(shadows(&[]), Vec::<Shadow>::new());
    assert_eq!(overlaps(&[]), Vec::<Overlap>::new());

    let one = routes("GET /a only");
    assert_eq!(shadows(&one), Vec::<Shadow>::new());
    assert_eq!(overlaps(&one), Vec::<Overlap>::new());
}

#[test]
fn public_relation_records_support_expected_derives() {
    let shadow = Shadow { route: 2, by: 0 };
    assert_eq!(shadow.clone(), shadow);
    assert_ne!(shadow, Shadow { route: 2, by: 1 });
    assert_eq!(format!("{shadow:?}"), "Shadow { route: 2, by: 0 }");

    let overlap = Overlap {
        first: 0,
        second: 2,
        method: String::from("GET"),
        path: String::from("/a/1"),
    };
    assert_eq!(overlap.clone(), overlap);
    assert_ne!(
        overlap,
        Overlap {
            first: 0,
            second: 2,
            method: String::from("POST"),
            path: String::from("/a/1"),
        }
    );
    assert_eq!(
        format!("{overlap:?}"),
        "Overlap { first: 0, second: 2, method: \"GET\", path: \"/a/1\" }"
    );
}
