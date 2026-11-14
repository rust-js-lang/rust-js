// An empty `Vec` extended by a slice's `inspect()`, in a crate that reads
// capacities, reserves the slice's length, as std's `Inspect` of it
// hints: made in place, `$iter(v).map(..)`, JS's own, it knows no hint,
// and grew from 4 to 8, so it's given knowing its length, as a collect of
// it is (#1792, #1804).
fn main() {
    let items = [1u32, 2, 3, 4, 5];
    let mut seen: Vec<&u32> = Vec::new();
    seen.extend(items.iter().inspect(|x| print!("{x} ")));
    println!("capacity {}", seen.capacity());
    let words = [String::from("a"), String::from("b"), String::from("c")];
    let mut lengths: Vec<&String> = Vec::new();
    lengths.extend(words.iter().inspect(|w| print!("{} ", w.len())));
    println!("capacity {}", lengths.capacity());
}
