use std::collections::BTreeSet;

use crate::{
    model::{METHODS, Route, Seg, Ty},
    problem::Problem,
    text,
};

struct Token<'a> {
    text: &'a str,
    start: usize,
}

fn column(line: &str, byte_offset: usize) -> usize {
    line.get(..byte_offset)
        .map_or(1, |prefix| prefix.encode_utf16().count() + 1)
}

fn make_problem(code: &'static str, text: String, line: usize, col: usize) -> Problem {
    Problem {
        code,
        text,
        line,
        col,
    }
}

fn tokens(line: &str) -> Vec<Token<'_>> {
    let mut result = Vec::new();
    let mut start = None;

    for (offset, character) in line.char_indices() {
        if character == ' ' {
            if let Some(token_start) = start.take()
                && let Some(value) = line.get(token_start..offset)
            {
                result.push(Token {
                    text: value,
                    start: token_start,
                });
            }
        } else if start.is_none() {
            start = Some(offset);
        }
    }

    if let Some(token_start) = start
        && let Some(value) = line.get(token_start..)
    {
        result.push(Token {
            text: value,
            start: token_start,
        });
    }

    result
}

fn check_token_characters(
    token: &Token<'_>,
    line: &str,
    line_number: usize,
) -> Result<(), Problem> {
    for (offset, character) in token.text.char_indices() {
        if !('!'..='~').contains(&character) {
            let character_text = character.to_string();
            return Err(make_problem(
                "RS001",
                format!("unexpected character {}", text::show(&character_text)),
                line_number,
                column(line, token.start + offset),
            ));
        }
    }
    Ok(())
}

fn parse_methods(
    token: &Token<'_>,
    line: &str,
    line_number: usize,
) -> Result<Vec<String>, Problem> {
    if token.text == "*" {
        return Ok(METHODS.iter().map(|method| (*method).to_owned()).collect());
    }

    let mut seen = BTreeSet::new();
    let mut part_start = 0;

    for part in token.text.split(',') {
        let col = column(line, token.start + part_start);
        if part == "*" || !METHODS.contains(&part) {
            return Err(make_problem(
                "RS002",
                format!("invalid method {}", text::show(part)),
                line_number,
                col,
            ));
        }
        if !seen.insert(part) {
            return Err(make_problem(
                "RS003",
                format!("repeated method {}", text::show(part)),
                line_number,
                col,
            ));
        }
        part_start += part.len() + 1;
    }

    Ok(METHODS
        .iter()
        .filter(|method| seen.contains(**method))
        .map(|method| (*method).to_owned())
        .collect())
}

fn valid_parameter_name(name: &str) -> bool {
    let mut characters = name.chars();
    matches!(characters.next(), Some('a'..='z'))
        && characters.all(|character| {
            character.is_ascii_lowercase() || character.is_ascii_digit() || character == '_'
        })
}

fn valid_route_name(name: &str) -> bool {
    let mut characters = name.chars();
    matches!(characters.next(), Some('a'..='z' | 'A'..='Z'))
        && characters.all(|character| {
            character.is_ascii_alphanumeric() || matches!(character, '_' | '.' | '-')
        })
}

fn invalid_segment(
    segment: &str,
    line: &str,
    token: &Token<'_>,
    segment_offset: usize,
    line_number: usize,
) -> Problem {
    make_problem(
        "RS006",
        format!("invalid segment {}", text::show(segment)),
        line_number,
        column(line, token.start + segment_offset),
    )
}

fn parse_segment(
    segment: &str,
    line: &str,
    token: &Token<'_>,
    segment_offset: usize,
    is_last: bool,
    captures: &mut BTreeSet<String>,
    line_number: usize,
) -> Result<Seg, Problem> {
    let segment_col = column(line, token.start + segment_offset);

    if segment.is_empty() {
        return Err(make_problem(
            "RS005",
            String::from("empty segment"),
            line_number,
            segment_col,
        ));
    }

    if !segment.starts_with('{') {
        let valid_literal = segment.chars().all(|character| {
            character.is_ascii_alphanumeric() || matches!(character, '.' | '_' | '~' | '-')
        });
        if valid_literal {
            return Ok(Seg::Lit(segment.to_owned()));
        }
        return Err(invalid_segment(
            segment,
            line,
            token,
            segment_offset,
            line_number,
        ));
    }

    if !segment.ends_with('}') {
        return Err(invalid_segment(
            segment,
            line,
            token,
            segment_offset,
            line_number,
        ));
    }

    let Some(contents) = segment.get(1..segment.len().saturating_sub(1)) else {
        return Err(invalid_segment(
            segment,
            line,
            token,
            segment_offset,
            line_number,
        ));
    };
    if contents.contains('{') || contents.contains('}') {
        return Err(invalid_segment(
            segment,
            line,
            token,
            segment_offset,
            line_number,
        ));
    }

    if let Some(name) = contents.strip_suffix('*') {
        if !valid_parameter_name(name) {
            return Err(invalid_segment(
                segment,
                line,
                token,
                segment_offset,
                line_number,
            ));
        }
        if !is_last {
            return Err(make_problem(
                "RS008",
                String::from("rest segment must be last"),
                line_number,
                segment_col,
            ));
        }
        if !captures.insert(name.to_owned()) {
            return Err(make_problem(
                "RS009",
                format!("duplicate parameter {}", text::show(name)),
                line_number,
                column(line, token.start + segment_offset + 1),
            ));
        }
        return Ok(Seg::Rest {
            name: name.to_owned(),
        });
    }

    let (name, type_text) = match contents.split_once(':') {
        Some((name, type_text)) => (name, Some(type_text)),
        None => (contents, None),
    };
    if !valid_parameter_name(name) {
        return Err(invalid_segment(
            segment,
            line,
            token,
            segment_offset,
            line_number,
        ));
    }

    let ty = match type_text.unwrap_or("any") {
        "int" => Ty::Int,
        "slug" => Ty::Slug,
        "any" => Ty::Any,
        unknown => {
            let type_offset = segment_offset + 1 + name.len() + 1;
            return Err(make_problem(
                "RS007",
                format!("unknown type {}", text::show(unknown)),
                line_number,
                column(line, token.start + type_offset),
            ));
        }
    };

    if !captures.insert(name.to_owned()) {
        return Err(make_problem(
            "RS009",
            format!("duplicate parameter {}", text::show(name)),
            line_number,
            column(line, token.start + segment_offset + 1),
        ));
    }

    Ok(Seg::Param {
        name: name.to_owned(),
        ty,
    })
}

fn parse_pattern(
    token: &Token<'_>,
    line: &str,
    line_number: usize,
    captures: &mut BTreeSet<String>,
) -> Result<Vec<Seg>, Problem> {
    if !token.text.starts_with('/') {
        return Err(make_problem(
            "RS004",
            String::from("pattern must start with \"/\""),
            line_number,
            column(line, token.start),
        ));
    }
    if token.text == "/" {
        return Ok(Vec::new());
    }

    let mut segments = Vec::new();
    let mut segment_start = 1;

    for (offset, character) in token.text.char_indices().skip(1) {
        if character == '/' {
            let Some(segment) = token.text.get(segment_start..offset) else {
                return Err(invalid_segment("", line, token, segment_start, line_number));
            };
            let parsed = parse_segment(
                segment,
                line,
                token,
                segment_start,
                false,
                captures,
                line_number,
            )?;
            segments.push(parsed);
            segment_start = offset + 1;
        }
    }

    let Some(last_segment) = token.text.get(segment_start..) else {
        return Err(invalid_segment("", line, token, segment_start, line_number));
    };
    let last_segment_offset = if segment_start == token.text.len() {
        token.text.len().saturating_sub(1)
    } else {
        segment_start
    };
    let parsed = parse_segment(
        last_segment,
        line,
        token,
        last_segment_offset,
        true,
        captures,
        line_number,
    )?;
    segments.push(parsed);

    Ok(segments)
}

fn parse_line(
    line: &str,
    line_number: usize,
    names: &mut BTreeSet<String>,
) -> Result<Option<Route>, Problem> {
    let line_tokens = tokens(line);
    if line_tokens.is_empty()
        || line
            .chars()
            .find(|character| *character != ' ')
            .is_some_and(|character| character == '#')
    {
        return Ok(None);
    }

    let Some(method_token) = line_tokens.first() else {
        return Ok(None);
    };
    check_token_characters(method_token, line, line_number)?;
    let methods = parse_methods(method_token, line, line_number)?;

    let Some(pattern_token) = line_tokens.get(1) else {
        return Err(make_problem(
            "RS012",
            String::from("missing pattern"),
            line_number,
            line.encode_utf16().count() + 1,
        ));
    };
    check_token_characters(pattern_token, line, line_number)?;
    let mut captures = BTreeSet::new();
    let segs = parse_pattern(pattern_token, line, line_number, &mut captures)?;

    let Some(name_token) = line_tokens.get(2) else {
        return Err(make_problem(
            "RS012",
            String::from("missing name"),
            line_number,
            line.encode_utf16().count() + 1,
        ));
    };
    check_token_characters(name_token, line, line_number)?;
    if !valid_route_name(name_token.text) {
        return Err(make_problem(
            "RS010",
            format!("invalid name {}", text::show(name_token.text)),
            line_number,
            column(line, name_token.start),
        ));
    }
    if !names.insert(name_token.text.to_owned()) {
        return Err(make_problem(
            "RS011",
            format!("duplicate name {}", text::show(name_token.text)),
            line_number,
            column(line, name_token.start),
        ));
    }

    if let Some(extra_token) = line_tokens.get(3) {
        check_token_characters(extra_token, line, line_number)?;
        return Err(make_problem(
            "RS013",
            format!("unexpected token {}", text::show(extra_token.text)),
            line_number,
            column(line, extra_token.start),
        ));
    }

    Ok(Some(Route {
        line: line_number,
        methods,
        pattern: pattern_token.text.to_owned(),
        segs,
        name: name_token.text.to_owned(),
    }))
}

pub fn parse(text: &str) -> Result<Vec<Route>, Problem> {
    let mut routes = Vec::new();
    let mut names = BTreeSet::new();

    for (index, physical_line) in text.split_inclusive('\n').enumerate() {
        let without_lf = physical_line.strip_suffix('\n').unwrap_or(physical_line);
        let line = without_lf.strip_suffix('\r').unwrap_or(without_lf);
        if let Some(route) = parse_line(line, index + 1, &mut names)? {
            routes.push(route);
        }
    }

    Ok(routes)
}
