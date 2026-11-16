// A zip of a `cloned()` or `copied()` of generic code's source with a
// range: std zips it by index where its caller's source is
// `TrustedRandomAccess`, a slice's, though generic code can't say so, and
// reads each side's item `i` as it steps (ADR 0975). Moved through generic
// code, it's std's struct: each pair is the source's item and the range's,
// by `next`, `nth`, `fold` and `try_fold`, the struct's own steps
// (#1982).
fn pass<I>(it: I) -> I {
    it
}

fn zipped_cloned<'a, I: Iterator<Item = &'a u32> + DoubleEndedIterator + ExactSizeIterator + Clone>(src: I) {
    let mut a = src.cloned().zip(10..13u32);
    println!("cloned first {:?}", a.next());
    let mut b = pass(a);
    println!("cloned nth {:?}", b.nth(0));
    println!("cloned rest {:?}", b.collect::<Vec<_>>());
}

fn zipped_copied<'a, I: Iterator<Item = &'a u32> + DoubleEndedIterator + ExactSizeIterator + Clone>(src: I) {
    let mut a = src.copied().zip(20..24u32);
    println!("copied first {:?}", a.next());
    let mut b = pass(a);
    let tried: Option<u32> = b.try_fold(0, |sum, (x, y)| if y > 22 { None } else { Some(sum + x * 100 + y) });
    println!("copied tried {tried:?}");
    println!("copied sum {}", b.fold(0, |sum, (x, y)| sum + x * 100 + y));
}

// Deeper and on the other side: a zip of that zip, an `enumerate()` of a
// `copied()`, and a range zipped with the copies. A `map` of the copies
// zipped is refused in the plain build (ADR 1014); the copy matrix runs it.
fn zipped_twice<'a, I: Iterator<Item = &'a u32> + DoubleEndedIterator + ExactSizeIterator + Clone>(src: I) {
    let mut a = src.cloned().zip(10..13u32).zip(20..23u32);
    println!("twice first {:?}", a.next());
    let b = pass(a);
    println!("twice rest {:?}", b.collect::<Vec<_>>());
}

fn zipped_enumerated<'a, I: Iterator<Item = &'a u32> + DoubleEndedIterator + ExactSizeIterator + Clone>(src: I) {
    let mut a = src.copied().enumerate().zip(10..13u32);
    println!("enumerated first {:?}", a.next());
    let mut b = pass(a);
    println!("enumerated nth {:?}", b.nth(1));
}

fn zipped_on_the_right<'a, I: Iterator<Item = &'a u32> + DoubleEndedIterator + ExactSizeIterator + Clone>(src: I) {
    let mut a = (10..13u32).zip(src.copied());
    println!("right first {:?}", a.next());
    let b = pass(a);
    println!("right rest {:?}", b.collect::<Vec<_>>());
}

fn main() {
    let nums = [1u32, 2, 3, 4, 5];
    zipped_cloned(nums.iter());
    zipped_copied(nums.iter());
    zipped_cloned(nums[1..].iter());
    zipped_copied(nums[2..].iter());
    zipped_twice(nums.iter());
    zipped_enumerated(nums.iter());
    zipped_on_the_right(nums.iter());
}
