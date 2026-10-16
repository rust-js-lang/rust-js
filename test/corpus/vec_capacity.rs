// A `Vec`'s capacity, kept where the crate asks it (ADR 0419), as std
// promises it: at least what `with_capacity` and `reserve` gave, its
// length, and left by `clear`, `truncate` and `pop`.
fn main() {
    let mut v: Vec<u32> = Vec::with_capacity(10);
    println!("{} {}", v.len(), v.capacity() >= 10);
    v.extend([1, 2, 3]);
    let before = v.capacity();
    v.clear();
    println!("{}", v.capacity() == before);
    v.reserve(20);
    println!("{}", v.capacity() >= 20);
    v.push(7);
    v.shrink_to_fit();
    println!("{} {}", v.capacity() >= v.len(), v.capacity() < 20);
    let mut w = vec![1, 2, 3, 4];
    let c = w.capacity();
    w.truncate(1);
    w.pop();
    println!("{} {}", w.capacity() == c, w.len());
}
