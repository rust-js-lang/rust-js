//! A library's generic functions, given as they are to its generic code
//! from another crate: each takes generic code's forms, a slice's raw
//! pointer as a handle (ADR 1114).
pub trait Len {
    fn length(&self) -> usize;
}

impl Len for [u8] {
    fn length(&self) -> usize {
        self.len()
    }
}

pub fn measured<T: ?Sized + Len>(p: *mut T) -> *mut T {
    println!("{}", unsafe { (*p).length() });
    p
}

pub fn apply_ptr<T: ?Sized, F: Fn(*mut T) -> *mut T>(p: *mut T, f: F) -> *mut T {
    f(p)
}

pub fn view<T: ?Sized>(r: &mut T) -> &T {
    &*r
}

pub fn replace_with<T>(slot: &mut T, with: T) -> T {
    std::mem::replace(slot, with)
}

pub fn apply_mut<T, F: Fn(&mut T, T) -> T>(slot: &mut T, with: T, f: F) -> T {
    f(slot, with)
}
