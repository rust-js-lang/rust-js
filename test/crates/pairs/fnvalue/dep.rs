//! A generic function, whose dictionary a function value of it must bind.
pub fn duplicate<T: Clone>(x: T) -> (T, T) {
    (x.clone(), x)
}

/// One taking a `&mut` to a number, whose value must be the function itself,
/// given its dictionaries: an arrow calling it would take the number, not its
/// place.
pub fn add_to<T: Into<u32>>(total: &mut u32, by: T) {
    *total += by.into();
}
