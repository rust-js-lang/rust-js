// The native side of bench-shapes-harness.js (#1532's S0): `bench-shapes.rs`
// built as a library crate, `shapes`, in a release build, timed as the
// harness times its JS, one operation per process.
//
//   bench-shapes-native <shape> <n> <mode> [reps]
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
    let shape = args[1].as_str();
    let n: u32 = args[2].parse().expect("n");
    let mode = args[3].as_str();
    let reps: u32 = args.get(4).map_or(1, |r| r.parse().expect("reps"));
    let file = format!("shape-{shape}-{n}.json");
    let (ms, note) = match mode {
        "emit" => {
            let text = shapes::make(shape, n);
            std::fs::write(&file, &text).expect("write the document");
            (0.0, format!("{} chars", text.len()))
        }
        _ => {
            let text = std::fs::read_to_string(&file).expect("read the document");
            match mode {
                "readTyped" => timed(reps, || shapes::read_typed(shape, &text).to_string()),
                "writeTyped" => timed(reps, || shapes::write_typed(shape, &text).to_string()),
                "readValue" => timed(reps, || shapes::read_value(&text).is_object().to_string()),
                "writeValue" => {
                    let value = shapes::read_value(&text);
                    timed(reps, || shapes::write_value(&value).len().to_string())
                }
                other => panic!("no mode {other}"),
            }
        }
    };
    println!("{shape} {mode} n={n} {ms:.1} ms ({note})");
}
