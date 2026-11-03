// A guard of `&&`s, which MIR tests in parts, an `if` of each inside the
// one before, is one `if` of the guard, as written: in the `if` chain of a
// `match` whose failed guard goes on to an arm that does something, and in
// a `switch`'s case (#530).

fn noted(log: &mut Vec<u32>, n: u32) -> bool {
    log.push(n);
    n > 1
}

fn pick(name: &str, keep: bool, more: bool) -> u32 {
    match name {
        "A" => 1,
        "B" if keep && more => 2,
        "C" => 3,
        _ => 0,
    }
}

fn compared(name: &str, n: u32, m: u32) -> u32 {
    match name {
        "A" => 1,
        "B" if n > 1 && m > 2 && n < m => 2,
        "C" => 3,
        _ => 0,
    }
}

/// Each part of the guard runs once, in order, where the one before held.
fn called(code: u32, log: &mut Vec<u32>) -> u32 {
    match code {
        1 => 10,
        2 if noted(log, code) && noted(log, code + 5) => 20,
        3 => 30,
        _ => 0,
    }
}

/// A `switch`'s case: what follows the `match` after it.
fn cased(code: u32, a: bool, b: bool, log: &mut Vec<u32>) {
    match code {
        1 => log.push(1),
        2 if a && b => log.push(2),
        3 => log.push(3),
        _ => {}
    }
    log.push(0);
}

fn main() {
    for name in ["A", "B", "C", "D"] {
        for (keep, more) in [(true, true), (true, false), (false, true)] {
            print!("{} ", pick(name, keep, more));
        }
        for (n, m) in [(2, 3), (2, 1), (5, 4), (0, 9)] {
            print!("{} ", compared(name, n, m));
        }
        println!();
    }
    let mut log = Vec::new();
    for code in 0..4 {
        let value = called(code, &mut log);
        println!("{value} {log:?}");
    }
    let mut log = Vec::new();
    for code in 0..4 {
        for (a, b) in [(true, true), (true, false), (false, true)] {
            cased(code, a, b, &mut log);
        }
    }
    println!("{log:?}");
}
