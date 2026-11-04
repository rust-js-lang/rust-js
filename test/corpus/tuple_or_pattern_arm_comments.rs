// A comment above a later arm of a tuple's or-pattern, `// c\n (2, 3) |
// (3, 2) => {`, whose alternatives start inside their tuples (#617): the
// arms are a chain in the order written, `if (a === 1 || b === 1) { .. }
// else if (..) {`, not MIR's `if (a !== 1 && b !== 1)` of the arms after
// the first, its `continue`s; and the comment is first in the arm's `else
// if` block, or above its guard `if`, not between its test's operands.
// What runs is the same: each test is pure, each pair takes the same arm.
pub fn looped(pairs: &[(u32, u32)]) -> Vec<bool> {
    let mut seen = Vec::new();
    let mut hit = false;
    for &(a, b) in pairs {
        seen.push(hit);
        match (a, b) {
            (1, _) | (_, 1) => hit = true,
            // Two and three, either way.
            (2, 3) | (3, 2) => {
                seen.push(true);
            }
            _ => hit = false,
        }
    }
    seen.push(hit);
    seen
}

pub fn returned(a: u32, b: u32) -> u32 {
    match (a, b) {
        (1, _) | (_, 1) => 1,
        // Two and three, either way.
        (2, 3) | (3, 2) => {
            let c = a * b;
            c + 1
        }
        _ => 0,
    }
}

pub fn assigned(a: u32, b: u32, log: &mut Vec<u32>) {
    let value = match (a, b) {
        (1, _) | (_, 1) => 2,
        // Two and three, either way.
        (2, 3) | (3, 2) => {
            log.push(a);
            7
        }
        _ => a + b,
    };
    log.push(value);
}

fn main() {
    let mut pairs = Vec::new();
    for a in 0..5 {
        for b in 0..5 {
            pairs.push((a, b));
        }
    }
    println!("{:?}", looped(&pairs));
    let mut log = Vec::new();
    for &(a, b) in &pairs {
        log.push(returned(a, b));
        assigned(a, b, &mut log);
    }
    println!("{log:?}");
}
