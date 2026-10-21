use crate::model::{Route, Seg, Ty};

fn capture(name: &str, body: &str, named: bool) -> String {
    if named {
        format!("(?P<{name}>{body})")
    } else {
        format!("({body})")
    }
}

pub fn regex_of(route: &Route, named: bool) -> String {
    let mut rendered = Vec::new();

    for segment in &route.segs {
        let text = match segment {
            Seg::Lit(literal) => literal.replace('.', r"\."),
            Seg::Param { name, ty } => {
                let body = match ty {
                    Ty::Int => "[0-9]+",
                    Ty::Slug => "[a-z0-9]+(?:-[a-z0-9]+)*",
                    Ty::Any => "[^/]+",
                };
                capture(name, body, named)
            }
            Seg::Rest { name } => capture(name, "[^/]+(?:/[^/]+)*", named),
        };
        rendered.push(text);
    }

    format!("^/{}$", rendered.join("/"))
}
