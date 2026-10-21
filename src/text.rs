pub fn show(text: &str) -> String {
    let mut shown = String::from("\"");
    for character in text.chars() {
        if (' '..='~').contains(&character) {
            shown.push(character);
        } else {
            shown.push('?');
        }
    }
    shown.push('"');
    shown
}

pub fn line(key: &str, value: &str) -> String {
    let mut output = String::new();
    output.push_str(key);
    for _ in 0..10usize.saturating_sub(key.chars().count()) {
        output.push(' ');
    }
    output.push_str("  ");
    output.push_str(value);
    output.push('\n');
    output
}

pub fn plural(n: usize, word: &str) -> String {
    let suffix = if n == 1 { "" } else { "s" };
    format!("{n} {word}{suffix}")
}

pub fn table(header: &[&str], rows: &[Vec<String>]) -> Vec<String> {
    let columns = rows.iter().map(Vec::len).fold(header.len(), usize::max);
    let mut widths = vec![0usize; columns];

    for (index, cell) in header.iter().enumerate() {
        if let Some(width) = widths.get_mut(index) {
            *width = (*width).max(cell.len());
        }
    }
    for row in rows {
        for (index, cell) in row.iter().enumerate() {
            if let Some(width) = widths.get_mut(index) {
                *width = (*width).max(cell.len());
            }
        }
    }

    let mut output = Vec::new();
    let header_cells = header.to_vec();
    output.push(format_row(&header_cells, &widths));
    for row in rows {
        let cells = row.iter().map(String::as_str).collect::<Vec<_>>();
        output.push(format_row(&cells, &widths));
    }
    output
}

fn format_row(cells: &[&str], widths: &[usize]) -> String {
    let mut output = String::new();
    for (index, width) in widths.iter().enumerate() {
        if index > 0 {
            output.push_str("  ");
        }
        let cell = cells.get(index).copied().unwrap_or("");
        output.push_str(cell);
        for _ in 0..width.saturating_sub(cell.len()) {
            output.push(' ');
        }
    }
    while output.ends_with(' ') {
        output.pop();
    }
    output
}
