// The native side of bench-harness.js (#1542, #1532's S0): `bench.rs`
// built as a library crate, `bench`, in a release build, and timed as the
// harness times its JS, one operation per process.
//
//   bench-native <n> <mode> [reps]
//
// `emit` writes `doc-<n>.json`; `write`, `read`, `readValue` and
// `writeValue` print the best of `reps` runs' milliseconds. Peak memory is
// read by `/usr/bin/time -l`, as the harness's is.
use std::time::Instant;

/// The best of `reps` runs of `f`, in milliseconds, and what its last made.
fn timed(reps: u32, mut f: impl FnMut() -> String) -> (f64, String) {
    let mut best = f64::INFINITY;
    let mut out = String::new();
    for _ in 0..reps {
        let start = Instant::now();
        out = f();
        best = best.min(start.elapsed().as_secs_f64() * 1000.0);
    }
    (best, out)
}

fn main() {
    let args: Vec<String> = std::env::args().collect();
    let n: u32 = args[1].parse().expect("n");
    let mode = args[2].as_str();
    let reps: u32 = args.get(3).map_or(1, |r| r.parse().expect("reps"));
    let file = format!("doc-{n}.json");
    let (ms, note) = match mode {
        "emit" => {
            let text = bench::write(&bench::make(n));
            std::fs::write(&file, &text).expect("write the document");
            (0.0, format!("{} chars", text.len()))
        }
        "write" => {
            let doc = bench::make(n);
            timed(reps, || bench::write(&doc).len().to_string())
        }
        "read" => {
            let text = std::fs::read_to_string(&file).expect("read the document");
            timed(reps, || bench::count(&bench::read(&text)).to_string())
        }
        "readValue" => {
            let text = std::fs::read_to_string(&file).expect("read the document");
            timed(reps, || bench::read_value(&text).is_object().to_string())
        }
        "writeValue" => {
            let text = std::fs::read_to_string(&file).expect("read the document");
            let value = bench::read_value(&text);
            timed(reps, || bench::write_value(&value).len().to_string())
        }
        other => panic!("no mode {other}"),
    };
    println!("{mode} n={n} {ms:.1} ms ({note})");
}
