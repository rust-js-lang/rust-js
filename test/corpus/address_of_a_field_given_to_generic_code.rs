//@ native: wasm32
// A field's `&mut` given to generic code that lets it out as a raw pointer,
// through a closure's parameter or a reborrow: its pointer is the field's
// place, at the field's offset in its struct, the caller's handle on it
// carrying that offset (#1471, #1479). The caller makes it as the field's
// raw pointer, a reborrow `&mut *r` of `r = &mut s.n`, whose place `*r` is
// the field `r` was made of. And a field's address by a reference cast,
// `&outer.inner.b as *const u32`, as `&raw const outer.inner.b`'s.
#[repr(C)]
struct S {
    a: u8,
    n: u32,
}

#[repr(C)]
struct Inner {
    a: u8,
    b: u32,
}

#[repr(C)]
struct Outer {
    x: u16,
    inner: Inner,
}

fn ptr_via<T>(x: &mut T) -> *mut T {
    let f = |y: &mut T| y as *mut T;
    f(x)
}

fn reborrowed<T>(x: &mut T) -> *mut T {
    let y = &mut *x;
    y as *mut T
}

fn main() {
    let mut s = S { a: 1, n: 2 };
    let base = &raw const s as usize;
    println!("{}", ptr_via(&mut s.n) as usize - base);
    println!("{}", reborrowed(&mut s.n) as usize - base);
    println!("{}", reborrowed(&mut s.a) as usize - base);
    println!("{} {}", s.a, s.n);

    let outer = Outer { x: 1, inner: Inner { a: 2, b: 3 } };
    let base = &raw const outer as usize;
    let raw = &raw const outer.inner.b as usize - base;
    let cast = &outer.inner.b as *const u32 as usize - base;
    println!("{raw} {cast} {}", outer.x + outer.inner.a as u16);
}
