// A `let` of a tuple's `match` whose arms' failures share a last `_ =>
// fallback`, a variable, which reading again runs no code, gives it in
// each branch that fails to it, `} else { n = fallback; }`, and no labeled
// block (#567, decided on the user's behalf 2026-11-04). The same
// assignment runs at the same point of each path as MIR's does at the
// block's end. A computed `_`, a call, is still a labeled block.
fn named(test: bool, tests: Option<u32>, main: bool, fallback: u32, log: &mut Vec<u32>) -> u32 {
    let n = match (test, tests) {
        (true, Some(t)) => {
            log.push(t);
            t + 1
        }
        (false, _) if main => 7,
        _ => fallback,
    };
    log.push(n);
    n
}

fn main() {
    let mut log = Vec::new();
    for test in [true, false] {
        for tests in [Some(3), None] {
            for main in [true, false] {
                println!("{}", named(test, tests, main, 40, &mut log));
            }
        }
    }
    println!("{log:?}");
}
