// A `MaybeUninit`'s `as_ptr()` and `as_mut_ptr()`, a pointer to its value's
// place (ADR 0332): of a type parameter's array, smallvec's inline
// `MaybeUninit<A>`, cast to its first item's (ADR 0417); and of a number.
use std::mem::MaybeUninit;

trait Array {
    type Item;
}

impl<T, const N: usize> Array for [T; N] {
    type Item = T;
}

fn first<A: Array>(m: &MaybeUninit<A>) -> A::Item
where
    A::Item: Copy,
{
    unsafe { *(m.as_ptr() as *const A::Item) }
}

fn set_second<A: Array>(m: &mut MaybeUninit<A>, v: A::Item) {
    unsafe { (m.as_mut_ptr() as *mut A::Item).add(1).write(v) }
}

/// A cast between two types' pointers in generic code, which JS has at the
/// same place only where they're one type, made where it runs, rust-js's
/// error there (ADR 0407): not here, where it doesn't.
fn reinterpret<T, U>(p: *const T, taken: bool) -> Option<*const U> {
    if taken { Some(p as *const U) } else { None }
}

fn main() {
    let mut m = MaybeUninit::new([1u32, 2, 3]);
    set_second(&mut m, 9);
    let arr = unsafe { m.assume_init() };
    println!("{} {:?}", first(&MaybeUninit::new([4u8, 5])), arr);
    let mut x = MaybeUninit::<u32>::uninit();
    unsafe { x.as_mut_ptr().write(5) };
    println!("{}", unsafe { x.assume_init() });
    println!("{}", reinterpret::<u32, u8>(&5, false).is_none());
}
