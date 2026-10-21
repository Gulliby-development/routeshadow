use crate::model::{METHODS, Route, Seg};

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Stats {
    pub routes: usize,
    pub fixed: usize,
    pub dynamic: usize,
    pub rest: usize,
    pub any_method: usize,
    pub deepest: usize,
}

pub fn stats(routes: &[Route]) -> Stats {
    let mut result = Stats {
        routes: routes.len(),
        fixed: 0,
        dynamic: 0,
        rest: 0,
        any_method: 0,
        deepest: 0,
    };

    for route in routes {
        if route
            .segs
            .iter()
            .all(|segment| matches!(segment, Seg::Lit(_)))
        {
            result.fixed += 1;
        } else {
            result.dynamic += 1;
        }

        if matches!(route.segs.last(), Some(Seg::Rest { .. })) {
            result.rest += 1;
        }
        if METHODS
            .iter()
            .all(|method| route.methods.iter().any(|present| present == method))
        {
            result.any_method += 1;
        }
        result.deepest = result.deepest.max(route.segs.len());
    }

    result
}
