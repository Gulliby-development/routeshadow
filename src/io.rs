use crate::{error::RsError, text};

pub trait Io {
    fn read_stdin(&self) -> Result<Vec<u8>, String>;
    fn read_file(&self, path: &str) -> Result<Vec<u8>, String>;
    fn out(&self, text: &str);
    fn err(&self, text: &str);
}

impl Io
    for (
        fn() -> Result<Vec<u8>, String>,
        fn(&str) -> Result<Vec<u8>, String>,
        fn(&str),
        fn(&str),
    )
{
    fn read_stdin(&self) -> Result<Vec<u8>, String> {
        (self.0)()
    }

    fn read_file(&self, path: &str) -> Result<Vec<u8>, String> {
        (self.1)(path)
    }

    fn out(&self, text: &str) {
        (self.2)(text);
    }

    fn err(&self, text: &str) {
        (self.3)(text);
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Source {
    pub display: String,
    pub text: String,
}

pub fn read_source(io: &dyn Io, arg: &str) -> Result<Source, RsError> {
    let (display, bytes) = if arg == "-" {
        (
            String::from("<stdin>"),
            io.read_stdin()
                .map_err(|_| RsError(String::from("<stdin>: cannot read the file")))?,
        )
    } else {
        let display = text::show(arg);
        let bytes = io
            .read_file(arg)
            .map_err(|_| RsError(format!("{display}: cannot read the file")))?;
        (display, bytes)
    };

    let mut source_text = String::from_utf8(bytes)
        .map_err(|_| RsError(format!("{display}: input is not valid UTF-8")))?;
    if source_text.starts_with('\u{feff}') {
        source_text.drain(..'\u{feff}'.len_utf8());
    }

    Ok(Source {
        display,
        text: source_text,
    })
}
