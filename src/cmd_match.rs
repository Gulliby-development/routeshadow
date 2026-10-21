use crate::{error::RsError, io::Io, loader, matcher::find_all, model::METHODS, options, text};

pub const USAGE: &str = "routeshadow match FILE METHOD PATH [--all]";

pub fn run(args: &[String], io: &dyn Io) -> Result<i32, RsError> {
    let parsed = options::parse(args, &["--all"])?;
    if parsed.args.is_empty() {
        return Err(RsError(String::from("missing FILE")));
    }
    if parsed.args.len() < 2 {
        return Err(RsError(String::from("missing METHOD")));
    }
    if parsed.args.len() < 3 {
        return Err(RsError(String::from("missing PATH")));
    }
    if parsed.args.len() > 3 {
        return Err(RsError(String::from("too many arguments")));
    }

    let routes = loader::load(io, &parsed.args[0])?;
    let method = &parsed.args[1];
    if !METHODS.contains(&method.as_str()) {
        return Err(RsError(format!("invalid method {}", text::show(method))));
    }

    let path = &parsed.args[2];
    if !path.starts_with('/') || !path.chars().all(|character| character.is_ascii_graphic()) {
        return Err(RsError(format!("invalid path {}", text::show(path))));
    }

    let hits = find_all(&routes, method, path);
    let Some(first_hit) = hits.first() else {
        return Ok(1);
    };
    let Some(first_route) = routes.get(first_hit.index) else {
        return Ok(1);
    };

    let mut output = String::new();
    output.push_str(&text::line("route", &first_route.name));
    output.push_str(&text::line("line", &first_route.line.to_string()));
    output.push_str(&text::line("pattern", &first_route.pattern));
    for (name, value) in &first_hit.values {
        output.push_str(&text::line("param", &format!("{name}={value}")));
    }

    if parsed.flags.contains("--all") {
        for hit in hits.iter().skip(1) {
            if let Some(route) = routes.get(hit.index) {
                output.push_str(&text::line(
                    "then",
                    &format!("{} (line {})", route.name, route.line),
                ));
            }
        }
    }

    io.out(&output);
    Ok(0)
}
