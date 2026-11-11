// A map a variable holds, of a range, enumerated and stepped from its back:
// its index from the back is its length less one, so it's made knowing
// that, `$rangeLen`, as a range's always can, where it was refused, "a
// `Map<Range<u32>, ..>` as an `ExactSizeIterator`, which rust-js can't
// count" (#1354). By `rfold`, `next_back`, `rev()` and `rposition`'s like,
// of a closure that captures and of a function.
fn doubled(x: u32) -> u32 {
    x * 2
}

fn main() {
    let offset = 10;
    let shifted = (1..6u32).map(|x| x + offset);
    println!("{}", shifted.enumerate().rfold(0, |a, (i, x)| a + i as u32 * x));
    let twice = (1..6u32).map(doubled);
    println!("{:?}", twice.enumerate().next_back());
    let again = (0..4u32).map(doubled);
    println!("{:?}", again.enumerate().rev().take(2).collect::<Vec<_>>());
    let found = (3..9u32).map(|x| x % 4);
    println!("{:?}", found.enumerate().rfind(|&(_, x)| x == 0));
}
