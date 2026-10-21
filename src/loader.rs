use crate::{
    error::RsError,
    io::{Io, read_source},
    parser,
};

pub fn load(io: &dyn Io, arg: &str) -> Result<Vec<crate::model::Route>, RsError> {
    let source = read_source(io, arg)?;
    let routes = parser::parse(&source.text)
        .map_err(|problem| RsError(format!("{}: {}", source.display, problem.render())))?;

    if routes.is_empty() {
        return Err(RsError(format!("{}: no routes", source.display)));
    }

    Ok(routes)
}
