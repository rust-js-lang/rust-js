// The allocator (ADR 0417): `alloc(layout)` a new allocation, an array of
// slots, given as a pointer to its bytes and cast to its items'; `realloc`
// the same, grown; `dealloc` nothing JS's collector doesn't do; a `Vec` of
// its first item's pointer, the array. And `Layout`, `{ size, align }`.
use std::alloc::{self, Layout};
use std::ptr::{self, NonNull};

/// A growable buffer of `T`s on the heap, as smallvec's spilled one is.
struct Buf<T> {
    ptr: NonNull<T>,
    cap: usize,
    len: usize,
}

fn layout_array<T>(n: usize) -> Layout {
    Layout::from_size_align(std::mem::size_of::<T>() * n, std::mem::align_of::<T>()).unwrap()
}

impl<T> Buf<T> {
    fn with_capacity(cap: usize) -> Self {
        let layout = layout_array::<T>(cap);
        let ptr = NonNull::new(unsafe { alloc::alloc(layout) })
            .unwrap_or_else(|| alloc::handle_alloc_error(layout))
            .cast();
        Buf { ptr, cap, len: 0 }
    }

    fn push(&mut self, value: T) {
        if self.len == self.cap {
            let old = layout_array::<T>(self.cap);
            let new = layout_array::<T>(self.cap * 2);
            let grown = unsafe { alloc::realloc(self.ptr.as_ptr() as *mut u8, old, new.size()) };
            self.ptr = NonNull::new(grown).unwrap().cast();
            self.cap *= 2;
        }
        unsafe { ptr::write(self.ptr.as_ptr().add(self.len), value) };
        self.len += 1;
    }

    fn pop(&mut self) -> Option<T> {
        if self.len == 0 {
            return None;
        }
        self.len -= 1;
        Some(unsafe { ptr::read(self.ptr.as_ptr().add(self.len)) })
    }

    fn into_vec(self) -> Vec<T> {
        let me = std::mem::ManuallyDrop::new(self);
        unsafe { Vec::from_raw_parts(me.ptr.as_ptr(), me.len, me.cap) }
    }
}

impl<T> Drop for Buf<T> {
    fn drop(&mut self) {
        unsafe {
            for i in 0..self.len {
                ptr::drop_in_place(self.ptr.as_ptr().add(i));
            }
            alloc::dealloc(self.ptr.as_ptr() as *mut u8, layout_array::<T>(self.cap));
        }
    }
}

fn main() {
    let mut b: Buf<String> = Buf::with_capacity(1);
    for w in ["a", "b", "c"] {
        b.push(w.to_uppercase());
    }
    let last = b.pop();
    println!("{} {} {:?}", b.len, b.cap, last);
    let v = b.into_vec();
    println!("{v:?}");
    let mut n: Buf<u32> = Buf::with_capacity(2);
    n.push(7);
    drop(n);
    let l = layout_array::<u64>(3);
    println!("{l:?} {} {}", l.size(), Layout::from_size_align(8, 3).is_err());
}
