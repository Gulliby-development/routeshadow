use crate::{error::RsError, io::Io, loader, options, regex::regex_of};

pub const USAGE: &str = "routeshadow regex FILE [--named]";

pub fn run(args: &[String], io: &dyn Io) -> Result<i32, RsError> {
    let parsed = options::parse(args, &["--named"])?;
    let Some(file) = parsed.args.first() else {
        return Err(RsError(String::from("missing FILE")));
    };
    if parsed.args.len() > 1 {
        return Err(RsError(String::from("too many arguments")));
    }

    let routes = loader::load(io, file)?;
    let named = parsed.flags.contains("--named");
    let mut output = String::new();

    for route in routes {
        output.push_str(&route.method_text());
        output.push(' ');
        output.push_str(&regex_of(&route, named));
        output.push(' ');
        output.push_str(&route.name);
        output.push('\n');
    }

    io.out(&output);
    Ok(0)
}
