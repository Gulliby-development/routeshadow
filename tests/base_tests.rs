use std::collections::BTreeSet;

use routeshadow::{
    error::RsError,
    io::{Io, Source, read_source},
    options::{Parsed, parse},
    text,
};

#[test]
fn base_types_and_callback_tuple_are_public_and_forward_calls() {
    let error = RsError(String::from("problem"));
    assert_eq!(format!("{error:?}"), "RsError(\"problem\")");
    assert_eq!(error.clone(), error);

    let parsed = Parsed {
        flags: BTreeSet::from([String::from("--all")]),
        args: vec![String::from("routes.txt")],
    };
    assert_eq!(parsed.clone(), parsed);
    assert_eq!(
        format!("{parsed:?}"),
        "Parsed { flags: {\"--all\"}, args: [\"routes.txt\"] }"
    );

    let source = Source {
        display: String::from("\"routes.txt\""),
        text: String::from("GET / home"),
    };
    assert_eq!(source.clone(), source);
    assert_eq!(
        format!("{source:?}"),
        "Source { display: \"\\\"routes.txt\\\"\", text: \"GET / home\" }"
    );

    let io = make_io(api_stdin, api_file, api_out, api_err);
    let erased: &dyn Io = &io;
    assert_eq!(erased.read_stdin(), Ok(vec![97]));
    assert_eq!(erased.read_file("api.txt"), Ok(vec![98]));
    erased.out("stdout payload");
    erased.err("stderr payload");
}

#[test]
fn show_quotes_ascii_and_replaces_each_non_printable_or_non_ascii_scalar() {
    assert_eq!(text::show(""), "\"\"");
    assert_eq!(text::show(" A~"), "\" A~\"");
    assert_eq!(text::show("a\t\n\u{7f}é😀"), "\"a?????\"");
    assert_eq!(text::show("\"\\"), "\"\"\\\"");
}

#[test]
fn line_pads_short_keys_and_leaves_long_keys_untrimmed() {
    assert_eq!(text::line("static", "6"), "static      6\n");
    assert_eq!(text::line("any-method", "1"), "any-method  1\n");
    assert_eq!(text::line("longer-than-ten", "x"), "longer-than-ten  x\n");
    assert_eq!(text::line("", ""), "            \n");
}

#[test]
fn plural_uses_singular_only_for_one() {
    assert_eq!(text::plural(0, "route"), "0 routes");
    assert_eq!(text::plural(1, "route"), "1 route");
    assert_eq!(text::plural(2, "route"), "2 routes");
}

#[test]
fn table_aligns_columns_trims_trailing_spaces_and_handles_empty_or_ragged_rows() {
    assert_eq!(
        text::table(
            &["A", "NAME"],
            &[
                vec![String::from("123"), String::from("x")],
                vec![String::from("2"), String::from("longer")],
            ],
        ),
        vec![
            String::from("A    NAME"),
            String::from("123  x"),
            String::from("2    longer")
        ]
    );
    assert_eq!(
        text::table(&["LINE", "NAME"], &[]),
        vec![String::from("LINE  NAME")]
    );
    assert_eq!(text::table(&[], &[]), vec![String::from("")]);
    assert_eq!(
        text::table(&["A", "B"], &[vec![String::from("x"), String::from("")]]),
        vec![String::from("A  B"), String::from("x")]
    );
    assert_eq!(
        text::table(&["A", "B"], &[vec![String::from("x")]]),
        vec![String::from("A  B"), String::from("x")]
    );
}

#[test]
fn options_parse_preserves_arguments_and_deduplicates_flags() {
    assert_eq!(
        parse(&[], &[]),
        Ok(Parsed {
            flags: BTreeSet::new(),
            args: Vec::new(),
        })
    );
    assert_eq!(
        parse(
            &[
                String::from("file"),
                String::from("--all"),
                String::from("-"),
                String::from("--all"),
                String::from("-5"),
                String::from(""),
                String::from("--named"),
            ],
            &["--all", "--named"],
        ),
        Ok(Parsed {
            flags: BTreeSet::from([String::from("--all"), String::from("--named")]),
            args: vec![
                String::from("file"),
                String::from("-"),
                String::from("-5"),
                String::from(""),
            ],
        })
    );
}

#[test]
fn options_parse_rejects_unknown_and_non_exact_options_in_order() {
    assert_eq!(
        parse(
            &[String::from("--write"), String::from("--bad")],
            &["--all"],
        ),
        Err(RsError(String::from("unknown option \"--write\"")))
    );
    assert_eq!(
        parse(&[String::from("--")], &[]),
        Err(RsError(String::from("unknown option \"--\"")))
    );
    assert_eq!(
        parse(&[String::from("--help")], &[]),
        Err(RsError(String::from("unknown option \"--help\"")))
    );
    assert_eq!(
        parse(&[String::from("--all=true")], &["--all"]),
        Err(RsError(String::from("unknown option \"--all=true\"")))
    );
    assert_eq!(
        parse(&[String::from("--é😀")], &[]),
        Err(RsError(String::from("unknown option \"--??\"")))
    );
}

#[test]
fn read_source_reads_empty_stdin_without_touching_other_channels() {
    let io = make_io(
        empty_stdin,
        unexpected_file,
        unexpected_output,
        unexpected_output,
    );
    assert_eq!(
        read_source(&io, "-"),
        Ok(Source {
            display: String::from("<stdin>"),
            text: String::new(),
        })
    );
}

#[test]
fn read_source_removes_one_bom_and_preserves_crlf() {
    let io = make_io(
        unexpected_stdin,
        bom_crlf_file,
        unexpected_output,
        unexpected_output,
    );
    assert_eq!(
        read_source(&io, "routes.txt"),
        Ok(Source {
            display: String::from("\"routes.txt\""),
            text: String::from("GET / home\r\n"),
        })
    );
}

#[test]
fn read_source_removes_exactly_one_of_two_leading_boms() {
    let io = make_io(
        unexpected_stdin,
        multiple_boms_file,
        unexpected_output,
        unexpected_output,
    );
    assert_eq!(
        read_source(&io, "routes.txt"),
        Ok(Source {
            display: String::from("\"routes.txt\""),
            text: String::from("\u{feff}x\u{feff}"),
        })
    );
}

#[test]
fn read_source_uses_exact_unicode_path_and_preserves_valid_unicode() {
    let io = make_io(
        unexpected_stdin,
        unicode_file,
        unexpected_output,
        unexpected_output,
    );
    assert_eq!(
        read_source(&io, "é"),
        Ok(Source {
            display: String::from("\"?\""),
            text: String::from("é😀"),
        })
    );
}

#[test]
fn read_source_discards_underlying_stdin_and_file_read_errors() {
    let stdin_io = make_io(
        failing_stdin,
        unexpected_file,
        unexpected_output,
        unexpected_output,
    );
    assert_eq!(
        read_source(&stdin_io, "-"),
        Err(RsError(String::from("<stdin>: cannot read the file")))
    );

    let file_io = make_io(
        unexpected_stdin,
        failing_file,
        unexpected_output,
        unexpected_output,
    );
    assert_eq!(
        read_source(&file_io, "nowhere.txt"),
        Err(RsError(String::from(
            "\"nowhere.txt\": cannot read the file"
        )))
    );
}

#[test]
fn read_source_rejects_malformed_utf8_for_stdin_and_file() {
    let stdin_io = make_io(
        malformed_stdin,
        unexpected_file,
        unexpected_output,
        unexpected_output,
    );
    assert_eq!(
        read_source(&stdin_io, "-"),
        Err(RsError(String::from("<stdin>: input is not valid UTF-8")))
    );

    let file_io = make_io(
        unexpected_stdin,
        malformed_file,
        unexpected_output,
        unexpected_output,
    );
    assert_eq!(
        read_source(&file_io, "bad.txt"),
        Err(RsError(String::from(
            "\"bad.txt\": input is not valid UTF-8"
        )))
    );
}

#[test]
fn read_source_rejects_truncated_and_overlong_utf8() {
    let truncated_io = make_io(
        unexpected_stdin,
        truncated_file,
        unexpected_output,
        unexpected_output,
    );
    assert_eq!(
        read_source(&truncated_io, "truncated.txt"),
        Err(RsError(String::from(
            "\"truncated.txt\": input is not valid UTF-8"
        )))
    );

    let overlong_io = make_io(
        unexpected_stdin,
        overlong_file,
        unexpected_output,
        unexpected_output,
    );
    assert_eq!(
        read_source(&overlong_io, "overlong.txt"),
        Err(RsError(String::from(
            "\"overlong.txt\": input is not valid UTF-8"
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

fn api_stdin() -> Result<Vec<u8>, String> {
    Ok(vec![97])
}

fn api_file(path: &str) -> Result<Vec<u8>, String> {
    assert_eq!(path, "api.txt");
    Ok(vec![98])
}

fn api_out(text: &str) {
    assert_eq!(text, "stdout payload");
}

fn api_err(text: &str) {
    assert_eq!(text, "stderr payload");
}

fn unexpected_stdin() -> Result<Vec<u8>, String> {
    panic!("unexpected stdin read");
}

fn unexpected_file(path: &str) -> Result<Vec<u8>, String> {
    panic!("unexpected file read: {path}");
}

fn unexpected_output(_text: &str) {
    panic!("unexpected output");
}

fn empty_stdin() -> Result<Vec<u8>, String> {
    Ok(Vec::new())
}

fn bom_crlf_file(path: &str) -> Result<Vec<u8>, String> {
    assert_eq!(path, "routes.txt");
    Ok(vec![
        0xef, 0xbb, 0xbf, 71, 69, 84, 32, 47, 32, 104, 111, 109, 101, 13, 10,
    ])
}

fn multiple_boms_file(path: &str) -> Result<Vec<u8>, String> {
    assert_eq!(path, "routes.txt");
    Ok(vec![
        0xef, 0xbb, 0xbf, 0xef, 0xbb, 0xbf, 120, 0xef, 0xbb, 0xbf,
    ])
}

fn unicode_file(path: &str) -> Result<Vec<u8>, String> {
    assert_eq!(path, "é");
    Ok(vec![0xc3, 0xa9, 0xf0, 0x9f, 0x98, 0x80])
}

fn failing_stdin() -> Result<Vec<u8>, String> {
    Err(String::from("underlying stdin detail"))
}

fn failing_file(path: &str) -> Result<Vec<u8>, String> {
    assert_eq!(path, "nowhere.txt");
    Err(String::from("underlying file detail"))
}

fn malformed_stdin() -> Result<Vec<u8>, String> {
    Ok(vec![65, 255, 10])
}

fn malformed_file(path: &str) -> Result<Vec<u8>, String> {
    assert_eq!(path, "bad.txt");
    Ok(vec![65, 255, 10])
}

fn truncated_file(path: &str) -> Result<Vec<u8>, String> {
    assert_eq!(path, "truncated.txt");
    Ok(vec![226, 130])
}

fn overlong_file(path: &str) -> Result<Vec<u8>, String> {
    assert_eq!(path, "overlong.txt");
    Ok(vec![192, 175])
}
