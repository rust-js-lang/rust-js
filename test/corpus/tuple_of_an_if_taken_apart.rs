// A tuple of an `if`'s value taken apart, `let (a, b) = if c { (x, y) }
// else { (z, w) };`, as clippy's `needless_late_init` asks for, is a `let`
// of each part assigned in each branch, `let a; let b; if (c) { a = x; b =
// y; } else { .. }`, as upstream's trackHydrated writes it, no array made
// (React port gap rd45). Each branch's parts are made in their order.

fn noted(log: &mut Vec<String>, what: &str, n: u32) -> u32 {
    log.push(format!("{what} {n}"));
    n
}

fn expected(checkable: bool, initial_value: &str, initial_checked: bool) -> String {
    let (value_field, expected_value) = if checkable {
        ("checked", initial_checked.to_string())
    } else {
        ("value", initial_value.to_owned())
    };
    value_field.to_owned() + &expected_value
}

fn three(n: u32, log: &mut Vec<String>) -> u32 {
    let (a, b, c) = if n > 3 {
        (noted(log, "a", 1), noted(log, "b", n), noted(log, "c", 3))
    } else if n > 1 {
        (n, 2, 2)
    } else {
        (0, 0, n)
    };
    a + b + c
}

fn matched(n: Option<u32>) -> u32 {
    let (a, b) = match n {
        Some(x) => (x, x + 1),
        None => (0, 1),
    };
    a * b
}

fn main() {
    println!("{} {}", expected(true, "x", false), expected(false, "x", true));
    let mut log = Vec::new();
    println!("{} {} {}", three(5, &mut log), three(2, &mut log), three(1, &mut log));
    println!("{log:?}");
    println!("{} {}", matched(Some(3)), matched(None));
}
