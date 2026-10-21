use crate::{error::RsError, io::Io, loader, options, text};

pub const USAGE: &str = "routeshadow list FILE [--by-name]";

pub fn run(args: &[String], io: &dyn Io) -> Result<i32, RsError> {
    let parsed = options::parse(args, &["--by-name"])?;
    let Some(file) = parsed.args.first() else {
        return Err(RsError(String::from("missing FILE")));
    };
    if parsed.args.len() > 1 {
        return Err(RsError(String::from("too many arguments")));
    }

    let routes = loader::load(io, file)?;
    let mut ordered_routes: Vec<_> = routes.iter().collect();
    if parsed.flags.contains("--by-name") {
        ordered_routes.sort_by(|left, right| left.name.as_bytes().cmp(right.name.as_bytes()));
    }

    let rows = ordered_routes
        .iter()
        .map(|route| {
            vec![
                route.line.to_string(),
                route.method_text(),
                route.pattern.clone(),
                route.name.clone(),
            ]
        })
        .collect::<Vec<_>>();
    let mut output = String::new();
    for line in text::table(&["LINE", "METHODS", "PATTERN", "NAME"], &rows) {
        output.push_str(&line);
        output.push('\n');
    }
    io.out(&output);
    Ok(0)
}
