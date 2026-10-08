// A closure that changes what it captured, moved, its variable's only use,
// is the same function, as nothing reads the one it came from; and one that
// changes nothing is copied freely (ADR 0246).
fn main() {
    let mut n = 0;
    let f = move || {
        n += 1;
        n
    };
    let mut g = f;
    g();
    println!("{}", g());
    let k = 41;
    let h = move || k + 1;
    let h2 = h;
    println!("{} {}", h(), h2());
}
