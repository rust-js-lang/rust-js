//! A library's functions that let a reference they're given out as a raw
//! pointer: their callers, another crate's, give them the pointer to the
//! place (ADR 1114).
use std::mem::MaybeUninit;

pub fn ptr_from_mut<T: ?Sized>(r: &mut T) -> *mut T {
    r
}

pub fn slice_as_uninit_mut<T>(slice: &mut [T]) -> &mut [MaybeUninit<T>] {
    let ptr = ptr_from_mut::<[T]>(slice) as *mut [MaybeUninit<T>];
    unsafe { &mut *ptr }
}

pub fn same<T: ?Sized>(p: *mut T) -> *mut T {
    p
}
