// A unit function of `let … else`s that `return`, then an `if` whose
// branches end it: each branch ends where the function does, with no
// `return;` MIR joined there, as React's completeBoundary is written, `if
// (typeof shellTime === "number") { ..; setTimeout(reveal, ms); } else {
// ..; requestAnimationFrame(..); }` (rd262; ADR 0879, amended).
use std::collections::HashMap;

fn complete(ids: &HashMap<&str, u32>, outer: &str, inner: &str, log: &mut Vec<u32>) {
    let Some(&first) = ids.get(outer) else {
        // Gone already.
        return;
    };
    let Some(&second) = ids.get(inner) else {
        log.push(first);
        // Nothing to reveal.
        return;
    };
    log.push(second);
    if log.len() == 2 {
        // The first in the batch.
        if second > 3 {
            log.push(second * 10);
        } else {
            log.push(0);
        }
    }
}

fn main() {
    let ids = HashMap::from([("a", 1), ("b", 5), ("c", 2)]);
    for (outer, inner) in [("x", "a"), ("a", "x"), ("a", "b"), ("c", "a")] {
        let mut log = Vec::new();
        complete(&ids, outer, inner, &mut log);
        log.push(9);
        complete(&ids, outer, inner, &mut log);
        println!("{outer} {inner} {log:?}");
    }
}
