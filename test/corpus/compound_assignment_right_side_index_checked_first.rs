//@ run-fail: index out of bounds: the len is 2 but the index is 2
// `v[i] += { let x = v[i]; v.push(0); x }`: the right side runs first, so its
// own `v[i]` is checked before anything narrows `i`, and panics as native
// Rust's does (#1660).
fn grow(v: &mut Vec<i32>, i: usize) {
    v[i] += {
        let x = v[i];
        v.push(0);
        x
    };
}

fn main() {
    let mut v = vec![1, 2];
    grow(&mut v, 2);
    println!("{v:?}");
}
