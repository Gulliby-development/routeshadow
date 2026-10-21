use std::io::{Read, Write};

use routeshadow::{cli, io::Io};

struct RealIo;

impl Io for RealIo {
    fn read_stdin(&self) -> Result<Vec<u8>, String> {
        let mut bytes = Vec::new();
        std::io::stdin()
            .read_to_end(&mut bytes)
            .map_err(|_| String::new())?;
        Ok(bytes)
    }

    fn read_file(&self, path: &str) -> Result<Vec<u8>, String> {
        std::fs::read(path).map_err(|_| String::new())
    }

    fn out(&self, text: &str) {
        let mut stdout = std::io::stdout().lock();
        let _ = stdout.write_all(text.as_bytes());
        let _ = stdout.flush();
    }

    fn err(&self, text: &str) {
        let mut stderr = std::io::stderr().lock();
        let _ = stderr.write_all(text.as_bytes());
        let _ = stderr.flush();
    }
}

fn main() {
    let args = std::env::args_os()
        .skip(1)
        .map(|argument| argument.to_string_lossy().into_owned())
        .collect::<Vec<_>>();
    std::process::exit(cli::run(&args, &RealIo));
}
