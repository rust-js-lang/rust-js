//@ run-fail: index out of bounds: the len is 3 but the index is 3
// A macro's repetition writes one index, `$v[$i]`, at one span for each
// item: `v[k]`, which the test shows in bounds, and `v[k + 1]`, which it
// doesn't. MIR's check is joined to THIR's index by its span alone, so it
// takes the one index there or none, never one of two: each check is kept,
// and `v[k + 1]` panics as native Rust's does (#1672).
macro_rules! pick {
    ($v:expr; $($i:expr),*) => {
        [$($v[$i]),*]
    };
}

fn pair(v: &[i32], k: usize) -> [i32; 2] {
    if k < v.len() { pick!(v; k, k + 1) } else { [0, 0] }
}

fn main() {
    let v = [1, 2, 3];
    println!("{:?}", pair(&v, 1));
    println!("{:?}", pair(&v, 2));
}
