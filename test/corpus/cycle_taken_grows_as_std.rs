// A `Vec` collected from a `take(n)` of an array's `iter().cycle()`, in a
// crate that reads a `Vec`'s capacity: std grows it by `Take`'s hint of
// the cycle's, `n` once it has items, so its capacity is what it holds.
// The runtime's cycle of an array's items was JS's own generator, whose
// `take` knows no hint, so the `Vec` grew by doubling, 8 where std's is 7
// (#1635).
fn capacity<T>(v: Vec<T>) -> usize {
    v.capacity()
}

fn main() {
    let a = [1, 2, 3];
    println!("{}", capacity(a.iter().cycle().take(7).collect()));
    println!("{}", capacity(a.iter().cycle().take(10).collect()));
    println!("{}", capacity(a.iter().cycle().take(2).collect()));
    println!("{}", capacity(a.iter().cycle().take(0).collect()));
    let seven: Vec<&i32> = a.iter().cycle().take(7).collect();
    println!("{seven:?} {}", seven.capacity());
    let none: [i32; 0] = [];
    println!("{}", capacity(none.iter().cycle().take(4).collect()));
    let owned: Vec<i32> = [4, 5].into_iter().cycle().take(7).collect();
    println!("{owned:?} {}", owned.capacity());
}
