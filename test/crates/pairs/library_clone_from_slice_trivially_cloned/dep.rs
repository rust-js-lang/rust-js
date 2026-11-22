//! A library's generic `clone_from_slice`, whose panic's message is
//! `copy_from_slice`'s where std clones its items as their bits: its
//! contract asks the consumer's `TTriviallyCloned`, of a type parameter
//! and of a type built of one, and its own generic code relays it (#2318).
pub fn refill<T: Clone>(into: &mut [T], from: &[T]) {
    into.clone_from_slice(from);
}

pub fn refill_tagged<T: Clone>(into: &mut [(T, u8)], from: &[(T, u8)]) {
    into.clone_from_slice(from);
}

pub fn relay<U: Clone>(into: &mut [U], from: &[U]) {
    refill(into, from);
}
