use crate::{error::RsError, io::Io, loader, options, relate};

pub const USAGE: &str = "routeshadow overlap FILE";

pub fn run(args: &[String], io: &dyn Io) -> Result<i32, RsError> {
    let parsed = options::parse(args, &[])?;
    let Some(file) = parsed.args.first() else {
        return Err(RsError("missing FILE".to_owned()));
    };
    if parsed.args.len() > 1 {
        return Err(RsError("too many arguments".to_owned()));
    }

    let routes = loader::load(io, file)?;
    let overlaps = relate::overlaps(&routes);
    if overlaps.is_empty() {
        return Ok(0);
    }

    let mut output = String::new();
    for overlap in overlaps {
        let (Some(first), Some(second)) = (routes.get(overlap.first), routes.get(overlap.second))
        else {
            continue;
        };
        output.push_str(&format!(
            "{} (line {}) and {} (line {}) both match {} {}\n",
            first.name, first.line, second.name, second.line, overlap.method, overlap.path
        ));
    }

    if output.is_empty() {
        Ok(0)
    } else {
        io.out(&output);
        Ok(1)
    }
}
