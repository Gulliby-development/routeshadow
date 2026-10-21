use crate::{builder, error::RsError, io::Io, loader, options, text};

pub const USAGE: &str = "routeshadow url FILE ROUTE [KEY=VALUE...]";

pub fn run(args: &[String], io: &dyn Io) -> Result<i32, RsError> {
    let parsed = options::parse(args, &[])?;
    let Some(file) = parsed.args.first() else {
        return Err(RsError(String::from("missing FILE")));
    };
    let Some(route_name) = parsed.args.get(1) else {
        return Err(RsError(String::from("missing ROUTE")));
    };

    let routes = loader::load(io, file)?;
    let Some(route) = routes.iter().find(|route| route.name == *route_name) else {
        return Err(RsError(format!("unknown route {}", text::show(route_name))));
    };
    let assignments = parsed.args.get(2..).unwrap_or(&[]);
    let path = builder::build(route, assignments)?;
    io.out(&format!("{path}\n"));
    Ok(0)
}
