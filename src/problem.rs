#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Problem {
    pub code: &'static str,
    pub text: String,
    pub line: usize,
    pub col: usize,
}

impl Problem {
    pub fn render(&self) -> String {
        format!("{} {} at {}:{}", self.code, self.text, self.line, self.col)
    }
}
