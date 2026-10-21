use std::collections::BTreeSet;

use crate::{error::RsError, text};

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Parsed {
    pub flags: BTreeSet<String>,
    pub args: Vec<String>,
}

pub fn parse(argv: &[String], specs: &[&str]) -> Result<Parsed, RsError> {
    let mut flags = BTreeSet::new();
    let mut args = Vec::new();

    for argument in argv {
        if argument.starts_with("--") {
            if !specs.contains(&argument.as_str()) {
                return Err(RsError(format!("unknown option {}", text::show(argument))));
            }
            flags.insert(argument.clone());
        } else {
            args.push(argument.clone());
        }
    }

    Ok(Parsed { flags, args })
}
