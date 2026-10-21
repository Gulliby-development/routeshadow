use crate::lang;
use crate::model::{METHODS, Route};

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Shadow {
    pub route: usize,
    pub by: usize,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Overlap {
    pub first: usize,
    pub second: usize,
    pub method: String,
    pub path: String,
}

pub fn contains(x: &Route, y: &Route) -> bool {
    y.methods
        .iter()
        .all(|method| x.methods.iter().any(|present| present == method))
        && lang::inside(&y.segs, &x.segs)
}

pub fn shadows(routes: &[Route]) -> Vec<Shadow> {
    let mut result = Vec::new();

    for (route_index, route) in routes.iter().enumerate() {
        for (earlier_index, earlier) in routes.iter().take(route_index).enumerate() {
            if contains(earlier, route) {
                result.push(Shadow {
                    route: route_index,
                    by: earlier_index,
                });
                break;
            }
        }
    }

    result
}

pub fn overlaps(routes: &[Route]) -> Vec<Overlap> {
    let mut result = Vec::new();

    for (second_index, second) in routes.iter().enumerate() {
        for (first_index, first) in routes.iter().take(second_index).enumerate() {
            if contains(first, second) || contains(second, first) {
                continue;
            }

            let Some(method) = METHODS.iter().find(|method| {
                first.methods.iter().any(|present| present == *method)
                    && second.methods.iter().any(|present| present == *method)
            }) else {
                continue;
            };

            let Some(segments) = lang::witness(&first.segs, &second.segs) else {
                continue;
            };

            let path = if segments.is_empty() {
                "/".to_owned()
            } else {
                format!("/{}", segments.join("/"))
            };
            result.push(Overlap {
                first: first_index,
                second: second_index,
                method: (*method).to_owned(),
                path,
            });
        }
    }

    result
}
