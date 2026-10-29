// A guarded arm whose guard fails goes on to the arms after it, a `_`
// that does nothing, and then to what's written after the `match`: React
// DOM's clearContainerSparingly, `"LINK" if is_stylesheet(node) => {
// continue; } _ => {}` and then `container.remove_child(node)`, which a
// `<link>` that isn't a stylesheet reaches. Not a `switch` whose `default`
// holds what follows the `match`, past which the `"LINK"` case's `break`
// goes, skipping it (rd184).

fn clear(items: &[(&str, bool)]) -> Vec<String> {
    let mut out = Vec::new();
    let mut i = 0;
    while let Some(&(name, keep)) = items.get(i) {
        i += 1;
        match name {
            "HTML" | "HEAD" | "BODY" => {
                out.push(format!("nested {name}"));
                continue;
            }
            "SCRIPT" | "STYLE" => {
                continue;
            }
            "LINK" if keep => {
                continue;
            }
            _ => {}
        }
        out.push(format!("removed {name}"));
    }
    out
}

fn codes(items: &[(u32, bool)]) -> Vec<u32> {
    let mut out = Vec::new();
    for &(code, keep) in items {
        match code {
            1 | 2 => {
                out.push(code * 10);
                continue;
            }
            3 => continue,
            4 if keep => continue,
            _ => {}
        }
        out.push(code);
    }
    out
}

fn kinds(items: &[(&str, bool)]) -> Vec<String> {
    let mut out = Vec::new();
    for &(name, keep) in items {
        match name {
            "A" => {
                out.push("a".to_string());
                continue;
            }
            "B" if keep => continue,
            "C" => {}
            "D" if keep => out.push("kept d".to_string()),
            _ => out.push(format!("other {name}")),
        }
        out.push(format!("tail {name}"));
    }
    out
}

fn main() {
    let items = [
        ("HTML", false),
        ("SCRIPT", false),
        ("LINK", true),
        ("LINK", false),
        ("DIV", false),
        ("STYLE", true),
    ];
    println!("{:?}", clear(&items));
    println!(
        "{:?}",
        kinds(&[
            ("A", true),
            ("B", true),
            ("B", false),
            ("C", true),
            ("D", true),
            ("D", false),
            ("E", false)
        ])
    );
    println!(
        "{:?}",
        codes(&[(1, false), (3, false), (4, true), (4, false), (5, true)])
    );
}
