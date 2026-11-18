// `map(Some)` of a range, of what can't look like `None`, is no map as it is
// where a `Box` holds one of its type, so it's a map made knowing its back,
// stepped from its back by an `enumerate()`, whose `next_back` counts by its
// source's length: the map's source made knowing its length too, as std's
// `Map` passes it on, not taken for one that knows it by its type (#2052).
fn main() {
    let weighted = (1..6u32)
        .map(Some)
        .enumerate()
        .rfold(0, |a, (i, x)| a + i as u32 * x.unwrap_or(0));
    println!("{weighted}");
    let last = (1..6u32).map(Some).enumerate().rev().take(2).collect::<Vec<_>>();
    println!("{last:?}");
    let held = Box::new((1..6u32).map(Some));
    println!("{}", held.flatten().rfold(0, |a, x| a * 10 + x));
}
