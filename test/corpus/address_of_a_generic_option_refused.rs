//@ compile-fail: a pointer to a value of a type parameter yet: generic code has one JS for each of `T`'s instances, and a `std::option::Option<P>` is the object it holds
// Generic code has one JS for every instance of its type parameter. A
// pointer to a `T` made of a reference to it, `r as *const T`, is
// `$ptrTo(r)` where `T` is an object and `$ptrOuter(r, 4)` where it's an
// `Option` of one, which JS holds as the object, past its tag: where a crate
// that asks an address gives it both, which can't be one JS, it's refused,
// naming #1114 and what to write instead. Where every instance agrees, as of
// `P` and `[P; 2]` alone, it isn't.
#[repr(C)]
struct P {
    a: u8,
    b: u32,
}

fn address<T>(r: &T) -> usize {
    r as *const T as usize
}

fn main() {
    let p = P { a: 1, b: 2 };
    let o = Some(P { a: 3, b: 4 });
    println!("{} {}", address(&p) - &raw const p as usize, address(&o) - &raw const o as usize);
    println!("{} {}", p.a + o.as_ref().unwrap().a, p.b);
}
