// Two `if`s the program wrote of one exit, `return false`, then what
// follows: each its own `if`, a comment between them where it's written,
// not one `if` of all their tests, which MIR's shared exit nests (rd237,
// React DOM's isScriptDataBlock).

fn is_data_block(kind: &str) -> bool {
    let lower = kind.to_lowercase();
    // Special keywords
    if lower == "module" || lower == "importmap" {
        return false;
    }
    // JavaScript MIME types
    if let "text/javascript" | "text/ecmascript" = lower.as_str() {
        return false;
    }
    // Any other type is a data block
    true
}

fn noted(n: u32) -> bool {
    println!("noted {n}");
    n % 3 == 0
}

// An exit that does more than return, its tests calls.
fn classified(n: u32, log: &mut Vec<u32>) -> u32 {
    if n == 0 || noted(n) {
        log.push(n);
        return 0;
    }
    if n > 50 {
        return 2;
    }
    log.push(n * 2);
    1
}

// An exit alike in both ways, MIR's one block: what's between them runs
// only where the test fails.
fn bumped(a: bool, b: bool, y: &mut u32) -> u32 {
    if a && b {
        return 7;
    }
    *y += 1;
    7
}

fn main() {
    for kind in ["module", "ImportMap", "text/javascript", "text/ecmascript", "application/json", ""] {
        println!("{kind:?} {}", is_data_block(kind));
    }
    let mut log = Vec::new();
    for n in [0, 3, 4, 51, 60, 7] {
        println!("{}", classified(n, &mut log));
    }
    println!("{log:?}");
    let mut y = 0;
    for (a, b) in [(true, true), (true, false), (false, true)] {
        println!("{} {y}", bumped(a, b, &mut y));
    }
}
