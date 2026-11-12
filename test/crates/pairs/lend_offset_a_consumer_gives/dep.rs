//! A library's generic code that lends a `&mut` from inside a `Result`, and
//! gives it back as a raw pointer: where its consumers' instances keep the
//! payload at different offsets, the offset is each caller's to give, a type
//! fact of the receiver, which the library's manifest asks of its consumers
//! (#1478). A library that asks an address itself, as one that asks none
//! holds its lends as no handle (#1474). A generic `OnceCell<T>`'s is in a
//! `$some` box, refused (#1513).

pub fn ok_ptr<T, E>(r: &mut Result<T, E>) -> *mut T {
    match r.as_mut() {
        Ok(x) => x as *mut T,
        Err(_) => std::ptr::null_mut(),
    }
}

pub fn err_ptr<T, E>(r: &mut Result<T, E>) -> *mut E {
    match r.as_mut() {
        Ok(_) => std::ptr::null_mut(),
        Err(e) => e as *mut E,
    }
}

/// An address the library asks itself, so it's a crate that asks one, whose
/// lends are handles on their places (#1474).
pub fn address_of<T>(value: &T) -> usize {
    value as *const T as usize
}
