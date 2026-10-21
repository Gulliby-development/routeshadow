use crate::{
    error::RsError,
    io::{Io, read_source},
    options, parser, stats, text,
};

pub const USAGE: &str = "routeshadow check FILE";

pub fn run(args: &[String], io: &dyn Io) -> Result<i32, RsError> {
    let parsed = options::parse(args, &[])?;
    let Some(file) = parsed.args.first() else {
        return Err(RsError(String::from("missing FILE")));
    };
    if parsed.args.len() > 1 {
        return Err(RsError(String::from("too many arguments")));
    }

    let source = read_source(io, file)?;
    let routes = match parser::parse(&source.text) {
        Ok(routes) => routes,
        Err(problem) => {
            io.out(&format!("{}\n1 problem\n", problem.render()));
            return Ok(1);
        }
    };

    if routes.is_empty() {
        return Err(RsError(format!("{}: no routes", source.display)));
    }

    let summary = stats::stats(&routes);
    let mut output = format!("ok, {}\n", text::plural(summary.routes, "route"));
    output.push_str(&text::line("static", &summary.fixed.to_string()));
    output.push_str(&text::line("dynamic", &summary.dynamic.to_string()));
    output.push_str(&text::line("rest", &summary.rest.to_string()));
    output.push_str(&text::line("any-method", &summary.any_method.to_string()));
    output.push_str(&text::line("deepest", &summary.deepest.to_string()));
    io.out(&output);
    Ok(0)
}
