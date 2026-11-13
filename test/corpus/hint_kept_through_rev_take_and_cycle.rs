// A `Vec` grown by an adapter of an adapter whose runtime form keeps std's
// hint, in a crate that reads a `Vec`'s capacity: a `rev()`, a `take(n)`
// and a `map()` of a `$Steps` pass their source's on, as std's do, so a
// `map()` of a string's `char_indices()` reversed, and a `take(n)` of a
// `cycle()` of a map of a `repeat_n`, grow as std's. The first was refused
// beside a case that reads a capacity (#1572).
fn capacity<T>(v: Vec<T>) -> usize {
    v.capacity()
}

fn main() {
    let s = "héllo wörld";
    let reversed: Vec<(usize, char)> = s.char_indices().map(|x| x).rev().collect();
    println!("{} {}", reversed.len(), reversed.capacity());
    println!("{}", capacity(s.chars().rev().map(|c| c).collect()));
    println!("{}", capacity(s.chars().map(|c| c).take(4).collect()));

    let cycled: Vec<u32> = std::iter::repeat_n(2u32, 3).map(|x| x + 1).cycle().take(7).collect();
    println!("{cycled:?} {}", cycled.capacity());
    let a = [1, 2, 3];
    println!("{}", capacity(a.iter().copied().rev().take(2).collect()));
    println!("{}", capacity(a.iter().filter(|x| **x > 1).take(2).collect()));
}
