use crate::{error::RsError, io::Io, text};

pub const USAGE: &str = "usage: routeshadow check FILE\n       routeshadow list FILE [--by-name]\n       routeshadow match FILE METHOD PATH [--all]\n       routeshadow shadow FILE\n       routeshadow overlap FILE\n       routeshadow url FILE ROUTE [KEY=VALUE...]\n       routeshadow regex FILE [--named]\n       routeshadow tree FILE\n";

pub fn run(argv: &[String], io: &dyn Io) -> i32 {
    let Some(command) = argv.first() else {
        io.err("routeshadow: missing command (check, list, match, shadow, overlap, url, regex or tree)\n");
        return 2;
    };

    if argv.len() == 1 && command == "--help" {
        io.out(USAGE);
        return 0;
    }

    let args = argv.iter().skip(1).cloned().collect::<Vec<_>>();
    let result = match command.as_str() {
        "check" => crate::cmd_check::run(&args, io),
        "list" => crate::cmd_list::run(&args, io),
        "match" => crate::cmd_match::run(&args, io),
        "shadow" => crate::cmd_shadow::run(&args, io),
        "overlap" => crate::cmd_overlap::run(&args, io),
        "url" => crate::cmd_url::run(&args, io),
        "regex" => crate::cmd_regex::run(&args, io),
        "tree" => crate::cmd_tree::run(&args, io),
        _ => {
            io.err(&format!(
                "routeshadow: unknown command {}\n",
                text::show(command)
            ));
            return 2;
        }
    };

    match result {
        Ok(code) => code,
        Err(RsError(message)) => {
            io.err(&format!("routeshadow: {message}\n"));
            2
        }
    }
}
