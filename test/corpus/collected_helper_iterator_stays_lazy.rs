// A runtime helper's iterator collected through a `map`, `extract_if`'s,
// a string's `split_terminator`'s and `matches`', stays lazy: each item
// tested, then mapped, in turn, as Rust's adapters step it, never all of
// them listed first, `Array.from($extractIf(..)).map(f)`, which would test
// every item before the first is mapped (ADR 0510, #1109).
fn main() {
    let mut v = vec![1, 2, 3, 4, 5, 6];
    let odd: Vec<i32> = v
        .extract_if(.., |x| {
            println!("test {x}");
            *x % 2 == 1
        })
        .map(|x| {
            println!("took {x}");
            x * 10
        })
        .collect();
    println!("{odd:?} {v:?}");
    let s = "a,b,,c";
    let parts: Vec<String> = s
        .split_terminator(',')
        .map(|p| {
            println!("part {p:?}");
            p.to_uppercase()
        })
        .collect();
    println!("{parts:?}");
    let words: Vec<usize> = "one two three"
        .matches(char::is_alphabetic)
        .map(|m| {
            println!("match {m}");
            m.len()
        })
        .take(4)
        .collect();
    println!("{words:?}");
}
