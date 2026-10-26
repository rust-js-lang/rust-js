// A `&mut` to a generic iterator an `Option` holds is the JS iterator it
// steps, as a pattern binds it (ADR 0921): `insert`, `get_or_insert` and
// `get_or_insert_with` give it so, as `as_mut()` does, where a number's
// or a generic `T`'s is a handle on its place.

fn inserted<T: IntoIterator>(slot: &mut Option<T::IntoIter>, x: T) -> Option<T::Item> {
    slot.insert(x.into_iter()).next()
}

fn by_as_mut<I: Iterator>(slot: &mut Option<I>) -> Option<I::Item> {
    slot.as_mut()?.next()
}

fn made<I: Iterator>(slot: &mut Option<I>, f: impl FnOnce() -> I) -> Option<I::Item> {
    slot.get_or_insert_with(f).next()
}

fn given<I: Iterator>(slot: &mut Option<I>, x: I) -> Option<I::Item> {
    slot.get_or_insert(x).next()
}

fn bumped<T: Default + std::ops::AddAssign + Copy>(slot: &mut Option<T>, by: T) -> T {
    let held = slot.get_or_insert_with(T::default);
    *held += by;
    *held
}

fn main() {
    let mut front = None;
    println!("{:?}", inserted(&mut front, vec![1, 2, 3]));
    println!("{:?}", front.map(|it| it.collect::<Vec<_>>()));

    let mut empty: Option<std::ops::Range<i32>> = None;
    println!("{:?} {:?}", made(&mut empty, || 4..7), empty);
    println!("{:?} {:?}", given(&mut empty, 9..10), empty);
    println!("{:?} {:?}", by_as_mut(&mut empty), empty);

    let mut words = None;
    println!("{:?}", made(&mut words, || "a b c".split(' ')));
    println!("{:?}", given(&mut words, "x".split(' ')));

    let mut count = None;
    println!("{} {}", bumped(&mut count, 2), bumped(&mut count, 3));
    let mut nested: Option<Option<i32>> = None;
    *nested.get_or_insert(None) = Some(4);
    println!("{nested:?} {count:?}");
}
