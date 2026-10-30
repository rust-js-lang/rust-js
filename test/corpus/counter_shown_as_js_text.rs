// A counter, an `f64` of a whole number written and one added or taken
// each time, is whole, never `-0`, and never further than 2^53 from zero,
// so `{}` of it is JS's text of it, `${index}` (rd266).

fn ids(n: usize) -> Vec<String> {
    let mut index = 0.0;
    let mut out = Vec::new();
    for _ in 0..n {
        out.push(format!("id-{index}"));
        index += 1.0;
    }
    let mut down = 2.0;
    while down > -3.0 {
        out.push(format!("down {down}"));
        down -= 1.0;
    }
    out
}

fn halves() -> Vec<String> {
    // Not a counter: half of one isn't whole.
    let mut x = 1.0;
    let mut out = Vec::new();
    for _ in 0..3 {
        out.push(format!("{x}"));
        x /= 2.0;
    }
    // Nor one of a large step, which may pass 1e21.
    let mut big = 0.0;
    for _ in 0..4 {
        out.push(format!("{big}"));
        big += 4e20;
    }
    // Nor one that starts past 2^53.
    let mut huge = 1e21;
    out.push(format!("{huge}"));
    huge -= 1.0;
    out.push(format!("{huge}"));
    // Nor `-0`.
    let mut zero = -0.0;
    out.push(format!("{zero}"));
    zero += 1.0;
    out.push(format!("{zero}"));
    out
}

fn main() {
    for line in ids(3).iter().chain(&halves()) {
        println!("{line}");
    }
}
