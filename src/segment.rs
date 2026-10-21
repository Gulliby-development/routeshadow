use crate::model::{Seg, Ty};

pub fn accepts(ty: Ty, text: &str) -> bool {
    match ty {
        Ty::Int => !text.is_empty() && text.chars().all(|character| character.is_ascii_digit()),
        Ty::Slug => {
            let mut has_character = false;
            let mut previous_was_hyphen = true;

            for character in text.chars() {
                if character == '-' {
                    if previous_was_hyphen {
                        return false;
                    }
                    previous_was_hyphen = true;
                } else if character.is_ascii_lowercase() || character.is_ascii_digit() {
                    has_character = true;
                    previous_was_hyphen = false;
                } else {
                    return false;
                }
            }

            has_character && !previous_was_hyphen
        }
        Ty::Any => !text.is_empty(),
    }
}

pub fn subset(x: &Seg, y: &Seg) -> bool {
    match (x, y) {
        (Seg::Lit(left), Seg::Lit(right)) => left == right,
        (Seg::Lit(text), Seg::Param { ty, .. }) => accepts(*ty, text),
        (Seg::Param { ty: left, .. }, Seg::Param { ty: right, .. }) => matches!(
            (left, right),
            (Ty::Int, Ty::Int | Ty::Slug | Ty::Any)
                | (Ty::Slug, Ty::Slug | Ty::Any)
                | (Ty::Any, Ty::Any)
        ),
        (Seg::Param { .. }, Seg::Lit(_)) | (Seg::Rest { .. }, _) | (_, Seg::Rest { .. }) => false,
    }
}

pub fn meet(x: &Seg, y: &Seg) -> Option<String> {
    if subset(x, y) {
        Some(sample(x))
    } else if subset(y, x) {
        Some(sample(y))
    } else {
        None
    }
}

pub fn sample(seg: &Seg) -> String {
    match seg {
        Seg::Lit(text) => text.clone(),
        Seg::Param { ty: Ty::Int, .. } => "1".to_owned(),
        Seg::Param { ty: Ty::Slug, .. } => "a".to_owned(),
        Seg::Param { ty: Ty::Any, .. } | Seg::Rest { .. } => "x".to_owned(),
    }
}
