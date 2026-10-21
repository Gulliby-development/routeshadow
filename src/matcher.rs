use crate::model::{Route, Seg};
use crate::segment::accepts;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Hit {
    pub index: usize,
    pub values: Vec<(String, String)>,
}

pub fn match_route(route: &Route, method: &str, path: &str) -> Option<Vec<(String, String)>> {
    if !route
        .methods
        .iter()
        .any(|route_method| route_method == method)
    {
        return None;
    }

    let path_without_leading_slash = path.strip_prefix('/')?;
    let path_segments: Vec<&str> = if path_without_leading_slash.is_empty() {
        Vec::new()
    } else {
        path_without_leading_slash.split('/').collect()
    };
    let mut path_segments = path_segments.into_iter();
    let mut route_segments = route.segs.iter().peekable();
    let mut values = Vec::new();

    while let Some(segment) = route_segments.next() {
        match segment {
            Seg::Lit(expected) => {
                let actual = path_segments.next()?;
                if actual.is_empty() || actual != expected {
                    return None;
                }
            }
            Seg::Param { name, ty } => {
                let actual = path_segments.next()?;
                if !accepts(*ty, actual) {
                    return None;
                }
                values.push((name.clone(), actual.to_owned()));
            }
            Seg::Rest { name } => {
                if route_segments.peek().is_some() {
                    return None;
                }

                let mut remaining = Vec::new();
                for actual in path_segments {
                    if actual.is_empty() {
                        return None;
                    }
                    remaining.push(actual);
                }
                if remaining.is_empty() {
                    return None;
                }
                values.push((name.clone(), remaining.join("/")));
                return Some(values);
            }
        }
    }

    if path_segments.next().is_some() {
        None
    } else {
        Some(values)
    }
}

pub fn find_all(routes: &[Route], method: &str, path: &str) -> Vec<Hit> {
    routes
        .iter()
        .enumerate()
        .filter_map(|(index, route)| {
            match_route(route, method, path).map(|values| Hit { index, values })
        })
        .collect()
}
