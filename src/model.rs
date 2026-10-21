#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Ty {
    Int,
    Slug,
    Any,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Seg {
    Lit(String),
    Param { name: String, ty: Ty },
    Rest { name: String },
}

pub const METHODS: [&str; 7] = ["GET", "POST", "PUT", "PATCH", "DELETE", "HEAD", "OPTIONS"];

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Route {
    pub line: usize,
    pub methods: Vec<String>,
    pub pattern: String,
    pub segs: Vec<Seg>,
    pub name: String,
}

impl Route {
    pub fn method_text(&self) -> String {
        if METHODS
            .iter()
            .all(|method| self.methods.iter().any(|present| present == method))
        {
            "*".to_owned()
        } else {
            self.methods.join(",")
        }
    }

    pub fn params(&self) -> Vec<String> {
        self.segs
            .iter()
            .filter_map(|segment| match segment {
                Seg::Param { name, .. } | Seg::Rest { name } => Some(name.clone()),
                Seg::Lit(_) => None,
            })
            .collect()
    }
}
