//@ compile-fail: `capacity()` of a `Vec` std may collect in place from a source a call hides (#1926)
// A function that reaches itself again, `forever` itself and `ping`
// through `pong`, gives no answer of what of its parameters it returns,
// as no walk of it ends (`returned_iterator`'s guard, #1934): a collect in
// place through one hides its source, and a `capacity()` of what it
// collects, which the same body reads, is refused where it's compiled
// (#1926). Neither runs, as it never would end.
#[allow(unconditional_recursion)]
fn forever(it: std::vec::IntoIter<u32>) -> std::vec::IntoIter<u32> {
    forever(it)
}

fn ping(it: std::vec::IntoIter<u32>, n: u32) -> std::vec::IntoIter<u32> {
    pong(it, n)
}

#[allow(unconditional_recursion)]
fn pong(it: std::vec::IntoIter<u32>, n: u32) -> std::vec::IntoIter<u32> {
    ping(it, n)
}

fn main() {
    let d: Vec<u32> = vec![1, 2].into_iter().collect();
    if d.is_empty() {
        let e: Vec<u32> = ping(vec![2].into_iter(), 0).collect();
        let f: Vec<u32> = forever(vec![1].into_iter()).collect();
        println!("{} {}", e.capacity(), f.capacity());
    }
    println!("{d:?}");
}
