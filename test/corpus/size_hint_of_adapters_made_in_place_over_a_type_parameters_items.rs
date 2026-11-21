// The hint of an iterator generic code holds whose adapters are made in
// place over a type parameter's items, `v.iter().skip(1).copied()` of a
// `&[T]`: each made knowing its hint in the function that asks, as a
// known type's are, `$skipping`, as their source is the slice's iterator
// whatever `T` is. It was refused, as generic code marked none, and JS's
// own `drop` keeps no hint (#2265). Callers of several `T`s, from each
// end and through a filter, against native.
pub fn f<T: Copy>(v: &[T]) -> (usize, Option<usize>) {
    let it = v.iter().skip(1).copied();
    it.size_hint()
}
pub fn g<T: Copy>(v: &[T]) -> (usize, Option<usize>) {
    let it = v.iter().skip(2).copied().rev();
    it.size_hint()
}
pub fn h<T: Clone + PartialEq>(v: &[T], x: &T) -> (usize, Option<usize>) {
    let mut it = v.iter().skip(1).filter(|y| *y != x).cloned();
    let before = it.size_hint();
    it.next();
    let _ = before;
    it.size_hint()
}
pub fn main() {
    println!("{:?} {:?}", f(&[1u8, 2, 3]), f(&["a", "b"]));
    println!("{:?} {:?}", f::<u64>(&[]), f(&[(1, 2)]));
    println!("{:?} {:?}", g(&[1u32, 2, 3, 4]), g(&[1.5f64]));
    println!(
        "{:?} {:?}",
        h(&[1, 2, 3, 2], &2),
        h(&["x".to_string()], &"y".to_string())
    );
}
