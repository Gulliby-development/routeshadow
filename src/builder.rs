use crate::{
    error::RsError,
    model::{Route, Seg, Ty},
    segment, text,
};

pub fn build(route: &Route, args: &[String]) -> Result<String, RsError> {
    let mut values = vec![None; route.segs.len()];

    for argument in args {
        let Some((key, value)) = argument.split_once('=') else {
            return Err(RsError(format!(
                "invalid argument {}",
                text::show(argument)
            )));
        };

        let Some(index) = route.segs.iter().position(|seg| match seg {
            Seg::Param { name, .. } | Seg::Rest { name } => name == key,
            Seg::Lit(_) => false,
        }) else {
            return Err(RsError(format!("unknown parameter {}", text::show(key))));
        };

        if values.get(index).is_some_and(Option::is_some) {
            return Err(RsError(format!("repeated parameter {}", text::show(key))));
        }

        let valid = !value.is_empty()
            && value.chars().all(|character| character.is_ascii_graphic())
            && match route.segs.get(index) {
                Some(Seg::Param { ty: Ty::Int, .. }) => segment::accepts(Ty::Int, value),
                Some(Seg::Param { ty: Ty::Slug, .. }) => segment::accepts(Ty::Slug, value),
                Some(Seg::Param { ty: Ty::Any, .. }) => !value.contains('/'),
                Some(Seg::Rest { .. }) => value.split('/').all(|part| !part.is_empty()),
                Some(Seg::Lit(_)) | None => false,
            };

        if !valid {
            return Err(RsError(format!(
                "invalid value {} for parameter {key}",
                text::show(value)
            )));
        }

        if let Some(slot) = values.get_mut(index) {
            *slot = Some(value.to_owned());
        }
    }

    for (index, seg) in route.segs.iter().enumerate() {
        if let Seg::Param { name, .. } | Seg::Rest { name } = seg
            && values.get(index).is_none_or(Option::is_none)
        {
            return Err(RsError(format!("missing parameter {name}")));
        }
    }

    let mut path = String::from("/");
    for (index, seg) in route.segs.iter().enumerate() {
        if index > 0 {
            path.push('/');
        }

        match seg {
            Seg::Lit(value) => path.push_str(value),
            Seg::Param { .. } | Seg::Rest { .. } => {
                if let Some(Some(value)) = values.get(index) {
                    path.push_str(value);
                }
            }
        }
    }

    Ok(path)
}
