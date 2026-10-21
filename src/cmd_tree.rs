use crate::{error::RsError, io::Io, loader, options, tree};

pub const USAGE: &str = "routeshadow tree FILE";

pub fn run(args: &[String], io: &dyn Io) -> Result<i32, RsError> {
    let parsed = options::parse(args, &[])?;
    if parsed.args.is_empty() {
        return Err(RsError(String::from("missing FILE")));
    }
    if parsed.args.len() > 1 {
        return Err(RsError(String::from("too many arguments")));
    }

    let Some(file) = parsed.args.first() else {
        return Err(RsError(String::from("missing FILE")));
    };
    let routes = loader::load(io, file)?;
    let mut output = String::new();
    for line in tree::lines(&routes) {
        output.push_str(&line);
        output.push('\n');
    }
    io.out(&output);
    Ok(0)
}
