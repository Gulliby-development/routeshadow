use routeshadow::{
    error::RsError,
    loader::load,
    model::{METHODS, Route, Seg, Ty},
    parser::parse,
    segment::{accepts, meet, sample, subset},
};

#[test]
fn public_model_types_keep_methods_and_capture_order() {
    assert_eq!(
        METHODS,
        ["GET", "POST", "PUT", "PATCH", "DELETE", "HEAD", "OPTIONS"]
    );

    let route = Route {
        line: 3,
        methods: vec![String::from("GET"), String::from("POST")],
        pattern: String::from("/a/{id:int}/{path*}"),
        segs: vec![
            Seg::Lit(String::from("a")),
            Seg::Param {
                name: String::from("id"),
                ty: Ty::Int,
            },
            Seg::Rest {
                name: String::from("path"),
            },
        ],
        name: String::from("items"),
    };
    assert_eq!(route.method_text(), "GET,POST");
    assert_eq!(
        route.params(),
        vec![String::from("id"), String::from("path")]
    );
    assert_eq!(route.clone(), route);
    let _debug = format!("{route:?}");

    let all_methods = Route {
        line: 1,
        methods: vec![
            String::from("GET"),
            String::from("POST"),
            String::from("PUT"),
            String::from("PATCH"),
            String::from("DELETE"),
            String::from("HEAD"),
            String::from("OPTIONS"),
        ],
        pattern: String::from("/"),
        segs: Vec::new(),
        name: String::from("root"),
    };
    assert_eq!(all_methods.method_text(), "*");
    assert_eq!(all_methods.params(), Vec::<String>::new());
    let copied_ty = Ty::Int;
    assert_eq!(copied_ty, Ty::Int);
}

#[test]
fn int_accepts_only_nonempty_ascii_digit_strings_without_numeric_conversion() {
    assert!(accepts(Ty::Int, "1"));
    assert!(accepts(Ty::Int, "000123456789012345678901234567890"));
    assert!(!accepts(Ty::Int, ""));
    assert!(!accepts(Ty::Int, "-1"));
    assert!(!accepts(Ty::Int, "4x"));
    assert!(!accepts(Ty::Int, "١"));
}

#[test]
fn slug_accepts_lowercase_groups_separated_by_single_hyphens() {
    assert!(accepts(Ty::Slug, "a"));
    assert!(accepts(Ty::Slug, "v2"));
    assert!(accepts(Ty::Slug, "a-b"));
    assert!(accepts(Ty::Slug, "2024-10-21"));
    assert!(!accepts(Ty::Slug, "A"));
    assert!(!accepts(Ty::Slug, "a--b"));
    assert!(!accepts(Ty::Slug, "-a"));
    assert!(!accepts(Ty::Slug, "a_b"));
    assert!(!accepts(Ty::Slug, "a-"));
    assert!(!accepts(Ty::Slug, ""));
}

#[test]
fn any_accepts_every_nonempty_text() {
    assert!(!accepts(Ty::Any, ""));
    assert!(accepts(Ty::Any, "Q?=#"));
    assert!(accepts(Ty::Any, "é"));
}

#[test]
fn subset_obeys_the_complete_segment_inclusion_hierarchy() {
    let int = Seg::Param {
        name: String::from("p"),
        ty: Ty::Int,
    };
    let renamed_int = Seg::Param {
        name: String::from("other"),
        ty: Ty::Int,
    };
    let slug = Seg::Param {
        name: String::from("p"),
        ty: Ty::Slug,
    };
    let any = Seg::Param {
        name: String::from("p"),
        ty: Ty::Any,
    };

    assert!(subset(
        &Seg::Lit(String::from("a")),
        &Seg::Lit(String::from("a"))
    ));
    assert!(!subset(
        &Seg::Lit(String::from("a")),
        &Seg::Lit(String::from("b"))
    ));
    assert!(subset(
        &Seg::Lit(String::from("001")),
        &Seg::Param {
            name: String::from("p"),
            ty: Ty::Int,
        }
    ));
    assert!(!subset(
        &Seg::Lit(String::from("A")),
        &Seg::Param {
            name: String::from("p"),
            ty: Ty::Slug,
        }
    ));
    assert!(subset(&int, &renamed_int));
    assert!(subset(&int, &slug));
    assert!(subset(&int, &any));
    assert!(subset(&slug, &slug));
    assert!(subset(&slug, &any));
    assert!(subset(&any, &any));
    assert!(!subset(&slug, &int));
    assert!(!subset(&any, &int));
    assert!(!subset(&any, &slug));
    assert!(!subset(&int, &Seg::Lit(String::from("1"))));
}

#[test]
fn meet_returns_a_sample_of_the_common_segment_language() {
    assert_eq!(
        meet(
            &Seg::Param {
                name: String::from("left"),
                ty: Ty::Any,
            },
            &Seg::Param {
                name: String::from("right"),
                ty: Ty::Int,
            }
        ),
        Some(String::from("1"))
    );
    assert_eq!(
        meet(
            &Seg::Param {
                name: String::from("p"),
                ty: Ty::Slug,
            },
            &Seg::Param {
                name: String::from("p"),
                ty: Ty::Any,
            }
        ),
        Some(String::from("a"))
    );
    assert_eq!(
        meet(
            &Seg::Lit(String::from("007")),
            &Seg::Param {
                name: String::from("p"),
                ty: Ty::Int,
            }
        ),
        Some(String::from("007"))
    );
    assert_eq!(
        meet(
            &Seg::Param {
                name: String::from("p"),
                ty: Ty::Int,
            },
            &Seg::Lit(String::from("007"))
        ),
        Some(String::from("007"))
    );
    assert_eq!(
        meet(
            &Seg::Lit(String::from("A")),
            &Seg::Param {
                name: String::from("p"),
                ty: Ty::Slug,
            }
        ),
        None
    );
    assert_eq!(
        meet(&Seg::Lit(String::from("a")), &Seg::Lit(String::from("b"))),
        None
    );
}

#[test]
fn sample_uses_the_prescribed_representative_for_each_segment() {
    assert_eq!(sample(&Seg::Lit(String::from(".."))), "..");
    assert_eq!(
        sample(&Seg::Param {
            name: String::from("p"),
            ty: Ty::Int,
        }),
        "1"
    );
    assert_eq!(
        sample(&Seg::Param {
            name: String::from("p"),
            ty: Ty::Slug,
        }),
        "a"
    );
    assert_eq!(
        sample(&Seg::Param {
            name: String::from("p"),
            ty: Ty::Any,
        }),
        "x"
    );
    assert_eq!(
        sample(&Seg::Rest {
            name: String::from("tail"),
        }),
        "x"
    );
}

#[test]
fn empty_space_and_comment_only_route_text_parses_to_no_routes() {
    assert_eq!(parse(""), Ok(Vec::new()));
    assert_eq!(parse(" \n# note 😀\r\n   # another\r"), Ok(Vec::new()));
}

#[test]
fn valid_route_files_preserve_full_records_and_physical_line_endings() {
    assert_eq!(
        parse("   GET   /a    n   \n# note"),
        Ok(vec![Route {
            line: 1,
            methods: vec![String::from("GET")],
            pattern: String::from("/a"),
            segs: vec![Seg::Lit(String::from("a"))],
            name: String::from("n"),
        }])
    );
    assert_eq!(
        parse(
            "# routes\r\n\r\nPOST,GET /{id:int}/{s:slug}/{a:any}/{tail*} Mixed_1.-\r\nGET / root\r"
        ),
        Ok(vec![
            Route {
                line: 3,
                methods: vec![String::from("GET"), String::from("POST")],
                pattern: String::from("/{id:int}/{s:slug}/{a:any}/{tail*}"),
                segs: vec![
                    Seg::Param {
                        name: String::from("id"),
                        ty: Ty::Int,
                    },
                    Seg::Param {
                        name: String::from("s"),
                        ty: Ty::Slug,
                    },
                    Seg::Param {
                        name: String::from("a"),
                        ty: Ty::Any,
                    },
                    Seg::Rest {
                        name: String::from("tail"),
                    },
                ],
                name: String::from("Mixed_1.-"),
            },
            Route {
                line: 4,
                methods: vec![String::from("GET")],
                pattern: String::from("/"),
                segs: Vec::new(),
                name: String::from("root"),
            },
        ])
    );
    assert_eq!(
        parse("* /./../A_0~-/{x} dots\n"),
        Ok(vec![Route {
            line: 1,
            methods: vec![
                String::from("GET"),
                String::from("POST"),
                String::from("PUT"),
                String::from("PATCH"),
                String::from("DELETE"),
                String::from("HEAD"),
                String::from("OPTIONS"),
            ],
            pattern: String::from("/./../A_0~-/{x}"),
            segs: vec![
                Seg::Lit(String::from(".")),
                Seg::Lit(String::from("..")),
                Seg::Lit(String::from("A_0~-")),
                Seg::Param {
                    name: String::from("x"),
                    ty: Ty::Any,
                },
            ],
            name: String::from("dots"),
        }])
    );
}

#[test]
fn load_reads_only_selected_stdin_and_normalizes_one_bom_before_parsing() {
    let io = make_io(
        stdin_bom_crlf,
        unexpected_file,
        unexpected_output,
        unexpected_output,
    );
    assert_eq!(
        load(&io, "-"),
        Ok(vec![Route {
            line: 1,
            methods: vec![String::from("GET")],
            pattern: String::from("/"),
            segs: Vec::new(),
            name: String::from("root"),
        }])
    );
}

#[test]
fn load_adds_context_for_parser_and_empty_source_errors() {
    let bad_file_io = make_io(
        unexpected_stdin,
        bad_route_file,
        unexpected_output,
        unexpected_output,
    );
    assert_eq!(
        load(&bad_file_io, "bad.txt"),
        Err(RsError(String::from(
            "\"bad.txt\": RS005 empty segment at 2:8"
        )))
    );

    let comment_file_io = make_io(
        unexpected_stdin,
        comment_only_file,
        unexpected_output,
        unexpected_output,
    );
    assert_eq!(
        load(&comment_file_io, "empty.txt"),
        Err(RsError(String::from("\"empty.txt\": no routes")))
    );

    let empty_stdin_io = make_io(
        empty_stdin,
        unexpected_file,
        unexpected_output,
        unexpected_output,
    );
    assert_eq!(
        load(&empty_stdin_io, "-"),
        Err(RsError(String::from("<stdin>: no routes")))
    );
}

#[test]
fn load_preserves_source_errors_sanitizes_paths_and_rejects_bad_boms() {
    let missing_file_io = make_io(
        unexpected_stdin,
        failing_file,
        unexpected_output,
        unexpected_output,
    );
    assert_eq!(
        load(&missing_file_io, "nowhere.txt"),
        Err(RsError(String::from(
            "\"nowhere.txt\": cannot read the file"
        )))
    );

    let unicode_path_io = make_io(
        unexpected_stdin,
        unicode_empty_file,
        unexpected_output,
        unexpected_output,
    );
    assert_eq!(
        load(&unicode_path_io, "é"),
        Err(RsError(String::from("\"?\": no routes")))
    );

    let malformed_stdin_io = make_io(
        malformed_stdin,
        unexpected_file,
        unexpected_output,
        unexpected_output,
    );
    assert_eq!(
        load(&malformed_stdin_io, "-"),
        Err(RsError(String::from("<stdin>: input is not valid UTF-8")))
    );

    let two_bom_io = make_io(
        two_bom_stdin,
        unexpected_file,
        unexpected_output,
        unexpected_output,
    );
    assert_eq!(
        load(&two_bom_io, "-"),
        Err(RsError(String::from(
            "<stdin>: RS001 unexpected character \"?\" at 1:1"
        )))
    );
}

fn make_io(
    stdin: fn() -> Result<Vec<u8>, String>,
    file: fn(&str) -> Result<Vec<u8>, String>,
    out: fn(&str),
    err: fn(&str),
) -> (
    fn() -> Result<Vec<u8>, String>,
    fn(&str) -> Result<Vec<u8>, String>,
    fn(&str),
    fn(&str),
) {
    (stdin, file, out, err)
}

fn unexpected_stdin() -> Result<Vec<u8>, String> {
    panic!("unexpected stdin read")
}

fn unexpected_file(path: &str) -> Result<Vec<u8>, String> {
    panic!("unexpected file read: {path}")
}

fn unexpected_output(_text: &str) {
    panic!("unexpected output")
}

fn stdin_bom_crlf() -> Result<Vec<u8>, String> {
    Ok(b"\xef\xbb\xbfGET / root\r\n".to_vec())
}

fn bad_route_file(path: &str) -> Result<Vec<u8>, String> {
    assert_eq!(path, "bad.txt");
    Ok(b"GET /a one\nPOST /b/ two".to_vec())
}

fn comment_only_file(path: &str) -> Result<Vec<u8>, String> {
    assert_eq!(path, "empty.txt");
    Ok(b"# nothing".to_vec())
}

fn empty_stdin() -> Result<Vec<u8>, String> {
    Ok(Vec::new())
}

fn failing_file(path: &str) -> Result<Vec<u8>, String> {
    assert_eq!(path, "nowhere.txt");
    Err(String::from("underlying read failure"))
}

fn unicode_empty_file(path: &str) -> Result<Vec<u8>, String> {
    assert_eq!(path, "é");
    Ok(b"# nothing".to_vec())
}

fn malformed_stdin() -> Result<Vec<u8>, String> {
    Ok(vec![65, 255, 10])
}

fn two_bom_stdin() -> Result<Vec<u8>, String> {
    Ok(b"\xef\xbb\xbf\xef\xbb\xbfGET / root".to_vec())
}
