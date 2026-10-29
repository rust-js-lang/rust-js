// React DOM's estimateBandwidth: a loop whose inner loop breaks, both
// skipping by a `continue` on a test of `||`, and whose own body ends in
// `if count > 10 { break; }`, which MIR has as `if (c) { break; } else {
// continue; }`: that `if` alone, no second `break` after its own, which
// failed tidy's "nothing after a jump" in a debug build (rd141, ADR 0730
// amended). And a `break` on a test of `||` in an inner loop, `if (t > end
// || s < 0) { break; }`.
fn average(times: &[f64], sizes: &[f64], end: f64) -> f64 {
    let mut count = 0;
    let mut total = 0.0;
    let mut i = 0;
    while i < times.len() {
        if times[i] == 0.0 || sizes[i] == 0.0 {
            i += 1;
            continue;
        }
        let mut j = i + 1;
        while j < times.len() {
            if times[j] > end {
                break;
            }
            if times[j] == 0.0 || sizes[j] == 0.0 {
                j += 1;
                continue;
            }
            total += times[j];
            j += 1;
        }
        i = j;
        count += 1;
        if count > 10 {
            break;
        }
    }
    total / f64::from(count)
}

fn until_past(times: &[f64], sizes: &[f64], end: f64) -> usize {
    let mut seen = 0;
    let mut i = 0;
    while i < times.len() {
        let mut j = i;
        while j < times.len() {
            if times[j] > end || sizes[j] < 0.0 {
                break;
            }
            j += 1;
        }
        seen += j - i;
        i = j + 1;
        if seen > 4 {
            break;
        }
    }
    seen
}

fn main() {
    let times = [1.0, 0.0, 2.0, 3.0, 9.0, 4.0, 0.5, 0.0, 2.5];
    let sizes = [1.0, 2.0, 0.0, 1.0, 1.0, -1.0, 1.0, 1.0, 1.0];
    println!("{}", average(&times, &sizes, 5.0));
    println!("{}", average(&times[..3], &sizes, 1.0));
    println!("{} {}", until_past(&times, &sizes, 5.0), until_past(&times, &sizes, 0.75));
}
