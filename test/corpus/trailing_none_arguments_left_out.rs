// #1842 (P0, ADR 0443, amended): a binding's trailing `None` is no
// argument, a dynamic one as a literal one is, so a native that counts its
// arguments answers as it does of the call written without them:
// `Math.max(1, undefined)` is NaN where `Math.max(1)` is 1. Each count's
// call, tested from the last, a `None` given before one that's there
// `undefined` as it was, and each argument made once, in Rust's order,
// before the first test. Natively, the binding's body says what JS's
// `Math.max` makes of what it's given.
#[cfg_attr(rust_js, rust_js::link_name = "Math.max")]
fn max(a: f64, b: Option<f64>, c: Option<f64>, d: Option<f64>, e: Option<f64>) -> f64 {
    // Each given, an `undefined` before one given NaN, as JS's is.
    let given = [b, c, d, e];
    let count = given.iter().rposition(Option::is_some).map_or(0, |i| i + 1);
    if given[..count].iter().any(Option::is_none) {
        return f64::NAN;
    }
    given[..count].iter().flatten().fold(a, |most, value| most.max(*value))
}

fn noted(log: &mut Vec<&'static str>, name: &'static str, value: Option<f64>) -> Option<f64> {
    log.push(name);
    value
}

fn most(b: Option<f64>, c: Option<f64>, d: Option<f64>, e: Option<f64>) -> f64 {
    max(1.0, b, c, d, e)
}

fn main() {
    let cases = [
        (None, None, None, None),
        (Some(2.0), None, None, None),
        (None, Some(3.0), None, None),
        (Some(2.0), Some(3.0), None, None),
        (None, None, None, Some(5.0)),
        (Some(2.0), Some(3.0), Some(4.0), Some(5.0)),
        (Some(0.5), None, Some(4.0), None),
    ];
    for (b, c, d, e) in cases {
        println!("{}", most(b, c, d, e));
    }
    // Made once each, in order, whichever call runs.
    let mut log = Vec::new();
    let shown = max(
        1.0,
        noted(&mut log, "b", Some(2.0)),
        noted(&mut log, "c", None),
        noted(&mut log, "d", None),
        noted(&mut log, "e", None),
    );
    println!("{shown} {}", log.join(","));
    println!("{}", max(1.0, None, None, None, None));
}
