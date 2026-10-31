// An `Option<bool>` that's `Some(true)`, written `is_some_and(|&w| w)` or
// `unwrap_or(false)`, is tested by its value's truth, as JS tests a flag
// that may be missing: `if (warned[key])`; where it's a value, a `bool`,
// `!!o` (ADR 0298, amended). `== Some(true)` is that value exactly, `o ===
// true` (the user's decision). And what a `!` turns, or a `while` tests,
// is read as an `if`'s test is.
use std::collections::HashMap;

fn warned(seen: &HashMap<&str, bool>, key: &str) -> bool {
    seen.get(key).is_some_and(|&warned| warned)
}

fn label(flag: Option<bool>) -> &'static str {
    if flag.is_some_and(|f| f) {
        "on"
    } else if flag == Some(true) {
        "never"
    } else if !flag.unwrap_or(false) {
        "off"
    } else {
        "never"
    }
}

fn countdown(flags: &[Option<bool>]) -> usize {
    let mut i = 0;
    while flags.get(i).copied().flatten().unwrap_or(false) {
        i += 1;
    }
    i
}

fn main() {
    let mut seen = HashMap::new();
    seen.insert("a", true);
    seen.insert("b", false);
    for key in ["a", "b", "c"] {
        println!("{key} {}", warned(&seen, key));
    }
    for flag in [Some(true), Some(false), None] {
        let off = !flag.unwrap_or(false);
        println!("{} {} {off}", label(flag), flag.is_some_and(|f| f));
    }
    println!("{}", countdown(&[Some(true), Some(true), None, Some(true)]));
    println!("{}", countdown(&[Some(false)]));
}
