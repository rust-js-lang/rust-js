// A `while` inside a `while`, the outer one's tail after the inner loop
// ending in a `break` of the outer: that tail after the inner loop, as
// written, `i = j; if (total > 10) { break; }`, not inside it, past its
// `break`, with a label `bb1:` for its `continue` (ADR 0702, amended).
fn sum_runs(xs: &[f64], limit: f64) -> f64 {
    let mut total = 0.0;
    let mut i = 0;
    while i < xs.len() {
        let mut j = i + 1;
        while j < xs.len() {
            if xs[j] > limit {
                break;
            }
            total += xs[j];
            j += 1;
        }
        i = j;
        if total > 10.0 {
            break;
        }
    }
    total
}

fn main() {
    println!("{}", sum_runs(&[1.0, 2.0, 3.0, 9.0, 4.0, 5.0], 8.0));
    println!("{}", sum_runs(&[1.0, 6.0, 7.0, 1.0, 2.0], 6.5));
    println!("{}", sum_runs(&[], 1.0));
}
