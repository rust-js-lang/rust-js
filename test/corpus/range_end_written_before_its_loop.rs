//@ run-fail: index out of bounds: the len is 2 but the index is 2
// A range whose end is written after it's made, `r.end = 3`, then looped
// over: its values reach the new end, past the slice, so each index's
// check is made. The MIR intervals keep a range's end only of one never
// written a field at a time (#1672 M3, ADR 1137).
fn total(v: &[u32]) -> u32 {
    let mut r = 0..v.len();
    r.end = 3;
    let mut t = 0;
    for i in r {
        t += v[i];
    }
    t
}

fn main() {
    println!("{}", total(&[1, 2]));
}
