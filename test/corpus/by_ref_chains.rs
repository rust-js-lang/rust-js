// A chain on `it.by_ref()` takes from `it` only what it's asked for, and
// leaves the rest in `it`, as Rust's does.
fn main() {
    let v = vec![1, 2, 3, 4, 5, 6, 7, 8];
    let mut it = v.iter();
    let first: Vec<_> = it.by_ref().take(2).collect();
    let small: Vec<_> = it.by_ref().take_while(|x| **x < 5).collect();
    println!("{first:?} {small:?} {:?}", it.next());
    let found = it.by_ref().find(|x| **x % 7 == 0);
    println!("{found:?} {:?}", it.as_slice());

    let mut words = "one two three four five".split(' ');
    let sum: usize = words.by_ref().map(|w| w.len()).take(3).sum();
    let any = words.by_ref().any(|w| w.starts_with('f'));
    println!("{sum} {any} {:?}", words.next());

    let mut chars = "abcdef".chars();
    let skipped = chars.by_ref().skip(1).next();
    let pos = chars.by_ref().position(|c| c == 'e');
    println!("{skipped:?} {pos:?} {:?}", chars.as_str());

    let mut nums = (10..20).collect::<Vec<u32>>().into_iter();
    let evens = nums.by_ref().filter(|n| n % 2 == 0).take(2).count();
    let rest: u32 = nums.sum();
    println!("{evens} {rest}");
}
