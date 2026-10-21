use crate::{error::RsError, io::Io, loader, options, relate};

pub const USAGE: &str = "routeshadow shadow FILE";

pub fn run(args: &[String], io: &dyn Io) -> Result<i32, RsError> {
    let parsed = options::parse(args, &[])?;
    let Some(file) = parsed.args.first() else {
        return Err(RsError("missing FILE".to_owned()));
    };
    if parsed.args.len() > 1 {
        return Err(RsError("too many arguments".to_owned()));
    }

    let routes = loader::load(io, file)?;
    let shadows = relate::shadows(&routes);
    if shadows.is_empty() {
        return Ok(0);
    }

    let mut output = String::new();
    for shadow in shadows {
        let (Some(route), Some(by)) = (routes.get(shadow.route), routes.get(shadow.by)) else {
            continue;
        };
        output.push_str(&format!(
            "{} (line {}) is shadowed by {} (line {})\n",
            route.name, route.line, by.name, by.line
        ));
    }

    if output.is_empty() {
        Ok(0)
    } else {
        io.out(&output);
        Ok(1)
    }
}
