// A guarded or-pattern's arm, `"vt-name" | "vt-update" if transitions`,
// which MIR tests once of each alternative, the guard in each, and tidy
// joins, `(name === "vt-name" && transitions) || (name === "vt-update" &&
// transitions)`: its cases, the guard's `if` in them once, as React DOM's
// diffHydratedProperties writes `case 'vt-name': case 'vt-update': if
// (enableViewTransition) { break; } // Fallthrough default: ..`, where it
// falls to a last `_` that does something, and goes past the `switch`
// where the `_` does nothing. Arms before it that do nothing, which MIR
// tests negated and tidy joins, `name !== "value" && name !== "checked"`,
// are each a case of their own (rd136, ADR 0905 amended). Of a guard that
// calls, which MIR's block tests apart in each alternative, the same. Each
// guard runs where Rust runs it, once its pattern matched.
use std::cell::RefCell;

thread_local! {
    static LOG: RefCell<Vec<String>> = const { RefCell::new(Vec::new()) };
}

/// A guard that says it ran.
fn on(name: &str, holds: bool) -> bool {
    LOG.with_borrow_mut(|log| log.push(format!("{name}?")));
    holds
}

fn note(what: &str) {
    LOG.with_borrow_mut(|log| log.push(what.to_string()));
}

fn extra(names: &[&str], transitions: bool) -> Vec<String> {
    let mut found = Vec::new();
    for &name in names {
        match name {
            // Controlled attributes are not validated.
            "value" => {}
            "checked" => {}
            "vt-name" | "vt-update" if on(name, transitions) => {
                // Expected from the server.
            }
            // Fallthrough
            _ => found.push(name.to_string()),
        }
    }
    found
}

/// Of a guard that reads what's matched only, which MIR's tree tests
/// alike of each alternative.
fn flagged(names: &[&str], transitions: bool) -> Vec<String> {
    let mut found = Vec::new();
    for &name in names {
        match name {
            "value" => {}
            "checked" => {}
            "vt-name" | "vt-update" if transitions => {
                // Expected from the server.
            }
            // Fallthrough
            _ => found.push(name.to_string()),
        }
    }
    found
}

fn unguarded(names: &[&str]) -> Vec<String> {
    let mut found = Vec::new();
    for &name in names {
        match name {
            "value" => {}
            "checked" => {}
            "vt-name" | "vt-update" => {
                // Expected from the server.
            }
            _ => found.push(name.to_string()),
        }
    }
    found
}

fn described(tag: u32, on_them: bool) -> u32 {
    match tag {
        1 => note("one"),
        2 => note("two"),
        3 | 4 | 5 if on("tag", on_them) => note("three to five"),
        _ => note("other"),
    }
    tag * 2
}

fn visited(tag: u32, on_them: bool) {
    match tag {
        1 => note("one"),
        2 => note("two"),
        3 | 4 if on("tag", on_them) => note("three or four"),
        _ => {}
    }
    note("after");
}

/// Of a `match` no `switch` is made of, the chain's one test of the arm,
/// its guard called once.
fn present(value: Option<u32>) -> u32 {
    match value {
        Some(1) | None if on("present", value.is_none()) => 10,
        _ => 20,
    }
}

/// Of alternatives that both match, `(1, 1)`, the guard called of each,
/// twice where the first fails.
fn either_one(a: u32, b: u32) -> u32 {
    match (a, b) {
        (1, _) | (_, 1) if on("either", a + b > 2) => 10,
        _ => 20,
    }
}

fn main() {
    let names = ["value", "checked", "vt-name", "vt-update", "class", "id"];
    for transitions in [true, false] {
        let found = extra(&names, transitions);
        let log = LOG.take();
        println!("extra {transitions}: {found:?} {log:?}");
    }
    println!("flagged: {:?} {:?}", flagged(&names, true), flagged(&names, false));
    println!("unguarded: {:?}", unguarded(&names));
    for value in [None, Some(1), Some(2)] {
        let got = present(value);
        println!("present {value:?}: {got} {:?}", LOG.take());
    }
    for (a, b) in [(1, 1), (1, 2), (2, 1), (2, 2)] {
        let got = either_one(a, b);
        println!("either {a} {b}: {got} {:?}", LOG.take());
    }
    for tag in 0..7 {
        for on_them in [true, false] {
            let doubled = described(tag, on_them);
            visited(tag, on_them);
            let log = LOG.take();
            println!("{tag} {on_them}: {doubled} {log:?}");
        }
    }
}
