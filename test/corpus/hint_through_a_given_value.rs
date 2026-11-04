// What generic code steps of what it's given, asked its `size_hint()`: a
// `rev()` of it, and a type parameter given as it is, `I: Clone`'s, passed
// on, of `Option`s stepped as such too, each the hint of what it steps, an
// array's items' or a range's, exact, as Rust's is (ADRs 0170, 0989; #382).
fn hint<J: Iterator>(it: J) -> (usize, Option<usize>) {
    it.size_hint()
}

fn passed_on<I: Iterator + Clone>(it: I) -> (usize, Option<usize>) {
    let copy = it.clone();
    let _ = copy.count();
    hint(it)
}

fn reversed_hint<I: DoubleEndedIterator>(it: I) -> (usize, Option<usize>) {
    it.rev().size_hint()
}

// Copied, of an exact size, its length given with it: a JS iterator of
// its own, stepping the one it's given, whose hint is that one's.
fn copied_hint<'a, I: ExactSizeIterator<Item = &'a u32> + Clone>(it: I) -> (usize, (usize, Option<usize>)) {
    let mut copied = it.copied();
    copied.next();
    (copied.len(), hint(copied.clone()))
}

fn main() {
    let v = vec![1u32, 2, 3];
    println!("{:?} {:?}", passed_on(v.iter()), passed_on(1..9u32));
    println!("{:?} {:?}", reversed_hint(v.iter()), reversed_hint(v.iter().skip(1)));
    println!("{:?} {:?}", copied_hint(v.iter()), copied_hint(v.iter().skip(1)));
    let options = vec![Some(1u32), None, Some(3)];
    println!(
        "{:?} {:?}",
        passed_on(options.iter().copied()),
        passed_on(options.iter())
    );
}
