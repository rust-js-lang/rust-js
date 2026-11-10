//! A library's generic functions whose `Some` of what looks like `None`
//! another crate gives, through a `dyn Fn`, a `fn` pointer and a call.
pub fn by_dyn<T>(t: T) -> Option<T> {
    Some(t)
}

pub fn by_pointer<T>(t: T) -> Option<T> {
    Some(t)
}

pub fn by_call<T>(t: T) -> Option<T> {
    Some(t)
}
