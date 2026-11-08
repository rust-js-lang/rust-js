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
/** The one event counter of every table's points: \`point()\`, which
 * panics where it's the run's \`k\`th. Its program has \`AtomicU32\` and the
 * atomic \`Ordering\` in scope. */
export const pointCounter = `
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

`;

export const pointsRunner = pointCounter + `fn run_points(name: &str, case: fn()) {
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

/** The runner of an async table's cases (#942): \`run_points!(name, case)\`
 * runs \`case()\`, an async fn, once a point, as \`run_points\` runs a
 * function, each run awaited to its end before the next. Natively each is
 * driven by \`block_on\`, its panic caught by \`catch_unwind\`; as JS, where
 * a future is a promise that runs as it's made (the guide's async), each is
 * awaited by \`js::settle\`, its panic its \`Err\`. Its program's \`main\`
 * runs \`run_all\`, which it writes of the cases, \`async fn run_all() {
 * run_points!(..); .. }\`. */
export const asyncPointsRunner = pointCounter + `
#[cfg(not(rust_js))]
fn block_on<F: std::future::Future>(f: F) -> F::Output {
    let mut f = std::pin::pin!(f);
    let mut cx = std::task::Context::from_waker(std::task::Waker::noop());
    loop {
        if let std::task::Poll::Ready(v) = f.as_mut().poll(&mut cx) {
            return v;
        }
    }
}

/** A run of \`case\`, and what its panic said. */
#[cfg(not(rust_js))]
macro_rules! ran {
    ($case:ident) => {
        std::panic::catch_unwind(|| block_on($case())).map_err(|p| p.downcast_ref::<&str>().copied().unwrap_or("?").to_string())
    };
}
#[cfg(rust_js)]
macro_rules! ran {
    ($case:ident) => {
        js::settle(js::promise($case())).await.map_err(|e| e.message())
    };
}

macro_rules! run_points {
    ($name:expr, $case:ident) => {{
        println!("case {}", $name);
        let mut k = 1;
        loop {
            POINTS.store(0, Ordering::Relaxed);
            POINT.store(k, Ordering::Relaxed);
            println!("point {k}");
            let ended = ran!($case);
            let fired = POINT.load(Ordering::Relaxed) == 0;
            POINT.store(0, Ordering::Relaxed);
            match ended {
                Ok(()) => println!("ended"),
                Err(message) => println!("panicked: {message}"),
            }
            if !fired || k == 200 {
                break;
            }
            k += 1;
        }
    }};
}

#[cfg(not(rust_js))]
fn main() {
    block_on(run_all());
}

#[cfg(rust_js)]
fn main() {
    js::spawn(Box::new(run_all()));
}
`;
