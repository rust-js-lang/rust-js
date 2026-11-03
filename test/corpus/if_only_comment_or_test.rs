// An `if` whose test is `a || b || c || (d && e)` and whose then-block is
// only a comment keeps its test as written, one `if` with the comment and
// an `else`, as React's clearHead writes it: not `if (a || b || c) { //
// retain } else if (d !== .. || e !== ..) { .. }`, the last disjunct
// turned into an `else if` of its negation (rd714).
fn is_marked_hoistable(name: &str) -> bool {
    name.starts_with('h')
}

fn rel(name: &str) -> String {
    if name.ends_with('k') {
        "StyleSheet".to_string()
    } else {
        "icon".to_string()
    }
}

pub fn clear_head(nodes: &[&str], removed: &mut Vec<String>) {
    let mut i = 0;
    while let Some(&current) = nodes.get(i) {
        let next = i + 1;
        let node_name = current.to_uppercase();
        if is_marked_hoistable(current)
            || node_name == "SCRIPT"
            || node_name == "STYLE"
            || (node_name == "LINK" && rel(current).to_lowercase() == "stylesheet")
        {
            // retain these nodes
        } else {
            removed.push(current.to_string());
        }
        i = next;
    }
}

fn main() {
    let mut removed = Vec::new();
    clear_head(&["hx", "script", "style", "link", "LINK", "div", "meta"], &mut removed);
    println!("{removed:?}");
}
