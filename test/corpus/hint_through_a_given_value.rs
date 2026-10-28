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

fn main() {
    let v = vec![1u32, 2, 3];
    println!("{:?} {:?}", passed_on(v.iter()), passed_on(1..9u32));
    println!("{:?} {:?}", reversed_hint(v.iter()), reversed_hint(v.iter().skip(1)));
    let options = vec![Some(1u32), None, Some(3)];
    println!(
        "{:?} {:?}",
        passed_on(options.iter().copied()),
        passed_on(options.iter())
    );
}
