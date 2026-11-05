// A guarded or-pattern whose guard calls nothing, which MIR tests after
// each alternative: tested once, after the alternatives, `(a === 1 || b ===
// 1) && flag`, as `(1, _) | (_, 1) if flag` is written, not `(a === 1 &&
// flag) || (b === 1 && flag)` (rd136).
fn either(a: u32, b: u32, flag: bool) -> u32 {
    match (a, b) {
        (1, _) | (_, 1) if flag => 10,
        _ => 0,
    }
}

fn present(value: Option<u32>, flag: bool) -> &'static str {
    match value {
        None | Some(1) if flag => "none or one",
        _ => "other",
    }
}

fn main() {
    for a in 0..3 {
        for b in 0..3 {
            println!("{a} {b}: {} {}", either(a, b, true), either(a, b, false));
        }
    }
    for given in [None, Some(1), Some(2)] {
        println!("{given:?}: {} {}", present(given, true), present(given, false));
    }
}
