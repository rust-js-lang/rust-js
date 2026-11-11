// A map whose function gives each item as it is in JS, `map(Some)` of what
// can't look like `None`, beneath what steps it from its back or counts
// it: a filter, an enumeration, a `flat_map`, a `rev()` and a flatten of
// one a variable holds, by `rfold`, `rfind`, `next_back` and `rev()`. Its
// JS is its source's, said so where it's left out, and its source is made
// knowing its back or its length, where it was refused (#1342); the
// matrix's rows hold a chain's flatten, which clippy has as a `filter_map`.
fn main() {
    let total = (1..6u32)
        .map(Some)
        .filter(Option::is_some)
        .rfold(0, |a, x| a * 10 + x.unwrap_or(0));
    println!("{total}");
    let found = (1..6u32)
        .map(Some)
        .filter(|x| x.is_some_and(|x| x % 2 == 0))
        .rfind(|_| true);
    println!("{found:?}");
    let weighted = (1..6u32)
        .map(Some)
        .enumerate()
        .rfold(0, |a, (i, x)| a + i as u32 * x.unwrap_or(0));
    println!("{weighted}");
    let last = (1..6u32).map(Some).enumerate().rev().take(2).collect::<Vec<_>>();
    println!("{last:?}");
    let pairs = (1..4u32)
        .map(Some)
        .flat_map(|x| [x, x])
        .rfold(0, |a, x| a * 10 + x.unwrap_or(0));
    println!("{pairs}");
    let mut evens = (1..6u32).map(Some).filter(|x| x.is_some_and(|x| x % 2 == 0));
    println!("{:?} {:?} {:?}", evens.next_back(), evens.next(), evens.next_back());
    let held = (1..6u32).map(Some);
    println!("{}", held.flatten().rfold(0, |a, x| a * 10 + x));
}
