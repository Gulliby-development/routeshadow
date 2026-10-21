use std::cmp::Ordering;

use crate::model::{Route, Seg, Ty};

#[derive(Debug, Clone, PartialEq, Eq)]
enum Key {
    Root,
    Literal(String),
    Int,
    Slug,
    Any,
    Rest,
}

impl Key {
    fn label(&self) -> &str {
        match self {
            Self::Root => "/",
            Self::Literal(text) => text,
            Self::Int => ":int",
            Self::Slug => ":slug",
            Self::Any => ":any",
            Self::Rest => "**",
        }
    }

    fn rank(&self) -> usize {
        match self {
            Self::Root => 0,
            Self::Literal(_) => 1,
            Self::Int => 2,
            Self::Slug => 3,
            Self::Any => 4,
            Self::Rest => 5,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
struct Node {
    key: Key,
    children: Vec<usize>,
    terminals: Vec<String>,
}

impl Node {
    fn new(key: Key) -> Self {
        Self {
            key,
            children: Vec::new(),
            terminals: Vec::new(),
        }
    }
}

fn key_for(segment: &Seg) -> Key {
    match segment {
        Seg::Lit(text) => Key::Literal(text.clone()),
        Seg::Param { ty: Ty::Int, .. } => Key::Int,
        Seg::Param { ty: Ty::Slug, .. } => Key::Slug,
        Seg::Param { ty: Ty::Any, .. } => Key::Any,
        Seg::Rest { .. } => Key::Rest,
    }
}

fn compare_keys(left: &Key, right: &Key) -> Ordering {
    match (left, right) {
        (Key::Literal(left), Key::Literal(right)) => left.as_bytes().cmp(right.as_bytes()),
        _ => left.rank().cmp(&right.rank()),
    }
}

/// Render routes as a deterministic, depth-first tree without trailing newlines.
pub fn lines(routes: &[Route]) -> Vec<String> {
    let mut arena = vec![Node::new(Key::Root)];

    for route in routes {
        let mut current = 0;
        for segment in &route.segs {
            let key = key_for(segment);
            let existing = arena.get(current).and_then(|node| {
                node.children
                    .iter()
                    .find_map(|child_id| match arena.get(*child_id) {
                        Some(child) if child.key == key => Some(*child_id),
                        _ => None,
                    })
            });

            current = if let Some(child_id) = existing {
                child_id
            } else {
                let child_id = arena.len();
                arena.push(Node::new(key));
                if let Some(parent) = arena.get_mut(current) {
                    parent.children.push(child_id);
                }
                child_id
            };
        }

        let annotation = format!("{} ({})", route.name, route.method_text());
        if let Some(node) = arena.get_mut(current) {
            node.terminals.push(annotation);
        }
    }

    for node_id in 0..arena.len() {
        let mut children = match arena.get(node_id) {
            Some(node) => node.children.clone(),
            None => continue,
        };
        children.sort_by(|left, right| match (arena.get(*left), arena.get(*right)) {
            (Some(left_node), Some(right_node)) => compare_keys(&left_node.key, &right_node.key),
            _ => Ordering::Equal,
        });
        if let Some(node) = arena.get_mut(node_id) {
            node.children = children;
        }
    }

    let mut result = Vec::new();
    let mut pending = vec![(0usize, 0usize)];
    while let Some((node_id, depth)) = pending.pop() {
        let Some(node) = arena.get(node_id) else {
            continue;
        };

        let mut line = String::new();
        for _ in 0..depth {
            line.push_str("  ");
        }
        line.push_str(node.key.label());
        if let Some(first) = node.terminals.first() {
            line.push_str(" = ");
            line.push_str(first);
            for terminal in node.terminals.iter().skip(1) {
                line.push_str(", ");
                line.push_str(terminal);
            }
        }
        result.push(line);

        let Some(child_depth) = depth.checked_add(1) else {
            continue;
        };
        for child_id in node.children.iter().rev() {
            pending.push((*child_id, child_depth));
        }
    }

    result
}
