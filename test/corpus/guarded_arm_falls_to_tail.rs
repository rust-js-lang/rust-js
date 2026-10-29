// A guarded arm whose guard fails goes on to the arms after it, a `_`
// that does nothing, and then to what's written after the `match`: React
// DOM's clearContainerSparingly, `"LINK" if is_stylesheet(node) => {
// continue; } _ => {}` and then `container.remove_child(node)`, which a
// `<link>` that isn't a stylesheet reaches. Not a `switch` whose `default`
// holds what follows the `match`, past which the `"LINK"` case's `break`
// goes, skipping it (rd184). Where the chain ends the loop's turn, a
// `switch` of the arms, what follows the `match` after it, as upstream
// writes it: each arm that went on to the turn's end `continue`s, and a
// guarded arm whose guard fails falls out of the `switch`, or into the
// `default` of a `_` that does something just after it (ADR 0636,
// amended).

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

/// A `_` that does something, the guarded arm just before it falling into
/// it; and one whose guarded arm isn't, an `if` chain still.
fn owned(items: &[(&str, bool)]) -> Vec<String> {
    let mut out = Vec::new();
    for &(name, keep) in items {
        match name {
            "A" => continue,
            "B" => {
                out.push("b".to_string());
                continue;
            }
            "C" if keep => continue,
            _ => out.push(format!("other {name}")),
        }
        out.push(format!("tail {name}"));
    }
    out
}

fn apart(items: &[(&str, bool)]) -> Vec<String> {
    let mut out = Vec::new();
    for &(name, keep) in items {
        match name {
            "C" if keep => continue,
            "A" => continue,
            "B" => {
                out.push("b".to_string());
                continue;
            }
            _ => out.push(format!("other {name}")),
        }
        out.push(format!("tail {name}"));
    }
    out
}

/// Of a function, each arm that doesn't go on leaving it: the `switch`,
/// what follows the `match` after it.
fn pick(name: &str, keep: bool) -> u32 {
    let mut n = 0;
    match name {
        "A" => return 1,
        "B" if keep => return 2,
        _ => {}
    }
    n += 5;
    n
}

/// Of a function, arms that go on past the `match`, each its case's
/// `break`.
fn counted(name: &str, keep: bool) -> u32 {
    let mut n = 0;
    match name {
        "A" => n += 1,
        "B" if keep => return 2,
        "C" => n += 3,
        _ => {}
    }
    n += 5;
    n
}

fn main() {
    for name in ["A", "B", "C", "D"] {
        for keep in [true, false] {
            println!("pick {name} {keep} {} {}", pick(name, keep), counted(name, keep));
        }
    }
    let named = [("A", true), ("B", false), ("C", true), ("C", false), ("D", false)];
    println!("{:?}", owned(&named));
    println!("{:?}", apart(&named));
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
