// A guarded arm that continues, before the other arms of a `match` in a
// loop, with what follows the `match` the rest of the turn: each arm that
// continues is its case's `continue`, and what follows the `match` is
// after the `switch`, as when the guarded arm is last (#518). MIR nests
// the blocks it jumps out of here, the `match`'s inside the one whose end
// is the rest of the turn, which the arm's `continue` skips too (#530).

fn is_sheet(name: &str) -> bool {
    !name.is_empty()
}

fn guarded_first(items: &[(&str, bool)]) -> Vec<String> {
    let mut out = Vec::new();
    for &(name, keep) in items {
        match name {
            "X" if keep => {
                out.push(format!("kept {name}"));
                continue;
            }
            "Y" => continue,
            _ => {}
        }
        out.push(format!("tail {name}"));
    }
    out
}

fn both_first(items: &[(&str, bool, bool)]) -> Vec<String> {
    let mut out = Vec::new();
    for &(name, a, b) in items {
        match name {
            "X" if a && b && is_sheet(name) => {
                out.push("x".to_string());
                continue;
            }
            "Y" => continue,
            _ => {}
        }
        out.push(format!("tail {name}"));
    }
    out
}

fn only_continues(items: &[(&str, bool)]) -> Vec<String> {
    let mut out = Vec::new();
    for &(name, keep) in items {
        match name {
            "X" if keep => continue,
            "Y" | "Z" => continue,
            _ => {}
        }
        out.push(format!("tail {name}"));
    }
    out
}

/// The `match` ends the turn: nothing past it is skipped.
fn ends_turn(items: &[(&str, bool)]) -> Vec<String> {
    let mut out = Vec::new();
    for &(name, keep) in items {
        match name {
            "X" if keep => {
                out.push(format!("kept {name}"));
                continue;
            }
            "Y" => continue,
            _ => out.push(format!("other {name}")),
        }
    }
    out
}

/// In a loop inside the one whose turn the arm continues.
fn nested(rows: &[&[(&str, bool)]]) -> Vec<String> {
    let mut out = Vec::new();
    for row in rows {
        for &(name, keep) in row.iter() {
            match name {
                "X" if keep => continue,
                "Y" => continue,
                _ => {}
            }
            out.push(format!("tail {name}"));
        }
        out.push("row".to_string());
    }
    out
}

/// A `match` that ends an inner loop's turn, in a loop with more past it:
/// what the outer turn does past the inner loop is no part of the inner
/// turn, which the arm's `continue` skips nothing of.
fn inner_ends_turn(rows: &[&[(&str, bool)]]) -> Vec<String> {
    let mut out = Vec::new();
    for row in rows {
        for &(name, keep) in row.iter() {
            match name {
                "X" if keep => {
                    out.push(format!("kept {name}"));
                    continue;
                }
                "Y" => continue,
                _ => out.push(format!("other {name}")),
            }
        }
        out.push("row".to_string());
    }
    out
}

fn main() {
    let named = [("X", true), ("X", false), ("Y", true), ("Z", false)];
    println!("{:?}", guarded_first(&named));
    println!("{:?}", only_continues(&named));
    println!("{:?}", ends_turn(&named));
    println!(
        "{:?}",
        both_first(&[
            ("X", true, true),
            ("X", true, false),
            ("Y", false, false),
            ("WXYZ", true, true)
        ])
    );
    println!("{:?}", nested(&[&named, &[("Y", false), ("W", true)]]));
    println!("{:?}", inner_ends_turn(&[&named, &[("W", true)]]));
}
