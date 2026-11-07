// Panic points (#817, #854): the one event counter and runner of every
// table that makes each point a panic can be the k-th, the iterator points
// table's (test/iterator-matrix.ts) and the unwind table's
// (test/unwind-points.ts). A program of either has `point()` at each of its
// points, and runs each case by `run_points`.

/** The runner (#817): each case run under `catch_unwind` once a point, its
 * `k`th point panicking, `point()`'s, until a run has no `k`th, which is
 * the case run plain. What a point is, is each table's: wherever its
 * program calls `point()`. One panic a run, so native Rust never aborts on
 * a second. Its program has `catch_unwind`, `AtomicU32` and the atomic
 * `Ordering` in scope. */
export const pointsRunner = `
/** The point that panics, the \`k\`th of a run, or none; and how many a
 * run has passed. */
static POINT: AtomicU32 = AtomicU32::new(0);
static POINTS: AtomicU32 = AtomicU32::new(0);

/** A point: where it's the run's \`k\`th, a panic, once. */
fn point() {
    let n = POINTS.fetch_add(1, Ordering::Relaxed) + 1;
    if n == POINT.load(Ordering::Relaxed) {
        POINT.store(0, Ordering::Relaxed);
        println!("panic at {n}");
        panic!("point");
    }
}

fn run_points(name: &str, case: fn()) {
    println!("case {name}");
    let mut k = 1;
    loop {
        POINTS.store(0, Ordering::Relaxed);
        POINT.store(k, Ordering::Relaxed);
        println!("point {k}");
        let ended = catch_unwind(case);
        let fired = POINT.load(Ordering::Relaxed) == 0;
        POINT.store(0, Ordering::Relaxed);
        match ended {
            Ok(()) => println!("ended"),
            Err(payload) => println!("panicked: {}", payload.downcast_ref::<&str>().copied().unwrap_or("?")),
        }
        if !fired || k == 200 {
            break;
        }
        k += 1;
    }
}
`;
