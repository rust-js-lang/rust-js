// Given where a `Copy` bound lets it be copied, as each call of a `Copy`
// `FnOnce` copies it, a closure that changes what it captured is cloned
// apart, as a `Copy` bound is a `Clone` one: each copy, its clone, counts
// on its own (ADRs 0246 and 0522, amended, #1211).
fn twice<F: FnMut() -> i32 + Copy>(mut f: F) -> (i32, i32) {
    let mut g = f;
    (f(), g())
}

fn outer<F: FnMut() -> i32 + Copy>(f: F) -> (i32, i32) {
    twice(f)
}

fn pairs<T: Copy>(t: T) -> (T, T) {
    (t, t)
}

fn main() {
    // One that writes nothing is `FnMut` by the bound alone: its copies
    // share nothing that changes.
    println!("{:?}", twice(|| 5));
    // Through generic code, and held in a tuple a `Copy` bound copies.
    let mut m = 10;
    println!(
        "{:?}",
        outer(move || {
            m += 1;
            m
        })
    );
    let mut k = 20;
    let (mut x, mut y) = pairs((
        move || {
            k += 1;
            k
        },
        0,
    ));
    println!("{} {}", (x.0)(), (y.0)());
    let mut n = 0;
    println!(
        "{:?}",
        twice(move || {
            n += 1;
            n
        })
    );
}
