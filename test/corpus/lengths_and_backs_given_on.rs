// What generic code asks the length, the back or the hint of, made where
// it's given: a `rev()` of a `skip()` of a range, an `enumerate()` of a
// range, and a map of what's given as it is, `I: Clone`'s, asked its hint
// (#385).
fn first<I: ExactSizeIterator<Item = u32>>(mut it: I) -> (usize, Option<u32>) {
    (it.len(), it.next())
}

fn counted<I: DoubleEndedIterator>(mut it: I) -> (bool, usize) {
    let had = it.next_back().is_some();
    (had, it.count())
}

fn hint<I: Iterator<Item = u32> + Clone>(it: I) -> (usize, Option<usize>) {
    it.map(|x| x + 1).size_hint()
}

fn main() {
    println!(
        "{:?} {:?}",
        first((0..10u32).skip(6).rev()),
        first((0..10u32).take(3).rev())
    );
    println!("{:?}", counted((1..5u32).enumerate()));
    let v = vec![1u32, 2, 3];
    println!("{:?} {:?}", hint(v.iter().copied()), hint(4..9u32));
}
