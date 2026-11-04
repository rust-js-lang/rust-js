// A `match` whose arms' failures share a last `_` the block computes, `_ =>
// compute(3)`, a call only the paths to it may run, is the chain of its
// arms, `if (test && tests != null && cond) { .. } else if (!test && main)
// { .. } else { entries = compute(3); }`, no labeled block: a later arm's
// test reads again what an earlier one tested, `test` then `!test`, which
// reads only what nothing changes (#583, the user's decision). The tuple
// it tests is made once, its call first, and the default runs only where
// no arm is taken.
fn compute(n: u32, log: &mut Vec<u32>) -> Vec<u32> {
    log.push(n);
    vec![n, n + 1]
}

fn flag(on: bool, log: &mut Vec<u32>) -> bool {
    log.push(100);
    on
}

fn entries(test: bool, tests: Option<u32>, cond: bool, main: bool, log: &mut Vec<u32>) -> Vec<u32> {
    let entries = match (flag(test, log), tests) {
        (true, Some(t)) if cond => vec![t],
        (false, _) if main => vec![9],
        _ => compute(3, log),
    };
    log.push(entries.len() as u32);
    entries
}

fn present(tests: Option<u32>, cond: bool, main: bool, log: &mut Vec<u32>) -> Vec<u32> {
    let entries = match tests {
        Some(t) if cond => vec![t],
        None if main => vec![9],
        _ => compute(4, log),
    };
    log.push(entries.len() as u32);
    entries
}

fn logged(test: bool, tests: Option<u32>, main: bool, log: &mut Vec<u32>) {
    match (test, tests) {
        (true, Some(t)) => log.push(t),
        (false, _) if main => log.push(9),
        _ => {
            compute(5, log);
        }
    }
    log.push(0);
}

fn main() {
    let mut log = Vec::new();
    for test in [true, false] {
        for tests in [Some(3), None] {
            for flag in [true, false] {
                println!("{:?}", entries(test, tests, flag, !flag, &mut log));
                println!("{:?}", entries(test, tests, flag, flag, &mut log));
                println!("{:?}", present(tests, flag, !flag, &mut log));
                logged(test, tests, flag, &mut log);
            }
        }
    }
    println!("{log:?}");
}
