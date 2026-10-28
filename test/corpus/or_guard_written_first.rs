// An `if` of `a && (b || c)` whose branch always leaves, before what the
// program wrote after it, is that `if` as written, `if (hydrating(x) &&
// (x === 4 || x > 9)) { .. }`, React's throwException's hydration path
// before its error path, not the error path inverted into it first (ADR
// 0790, amended).
fn hydrating(x: u32) -> bool {
    x > 3
}

fn throw_exception(x: u32, boundary: Option<u32>, log: &mut Vec<u32>) -> bool {
    log.push(0);
    // This is a regular error, not a Suspense wakeable.
    if hydrating(x) && (x == 4 || x > 9) {
        log.push(1);
        if let Some(b) = boundary {
            if b > 2 {
                log.push(b);
            }
            log.push(2);
        } else {
            log.push(3);
            return true;
        }
        return false;
    } else {
        // Otherwise, fall through to the error path.
    }

    log.push(4);
    if x == 0 {
        return true;
    }
    log.push(5);
    false
}

fn main() {
    let mut log = Vec::new();
    for x in [0, 1, 4, 5, 10] {
        for boundary in [None, Some(1), Some(3)] {
            let thrown = throw_exception(x, boundary, &mut log);
            log.push(u32::from(thrown) + 10);
        }
    }
    println!("{log:?}");
}
