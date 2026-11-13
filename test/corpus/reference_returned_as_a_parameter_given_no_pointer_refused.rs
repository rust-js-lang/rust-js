//@ compile-fail: a raw pointer to what a `&mut` a call lends refers to
// What `deinit_slice_mut` returns is its parameter, but `run`'s `headers`,
// whose signature lets no pointer out, is given no place's pointer: a box of
// a copy, maybe. So a raw pointer to it, `parse_headers_iter_uninit`'s,
// given its place's pointer (ADR 1114), is refused, as any call's cell is
// (#1513).
use std::mem::MaybeUninit;

#[derive(Clone, Copy, Debug)]
struct Header {
    name: &'static str,
}

struct Bytes {
    seen: *const &'static mut [MaybeUninit<Header>],
}

unsafe fn deinit_slice_mut<'a, 'b, T>(s: &'a mut &'b mut [T]) -> &'a mut &'b mut [MaybeUninit<T>] {
    let s: *mut &mut [T] = s;
    let s = s as *mut &mut [MaybeUninit<T>];
    unsafe { &mut *s }
}

fn parse_headers_iter_uninit(headers: &mut &'static mut [MaybeUninit<Header>], bytes: &mut Bytes) -> usize {
    bytes.seen = &raw const *headers;
    headers[0] = MaybeUninit::new(Header { name: "Host" });
    1
}

fn run(headers: &mut &'static mut [Header]) -> usize {
    let mut bytes = Bytes { seen: std::ptr::null() };
    parse_headers_iter_uninit(unsafe { deinit_slice_mut(headers) }, &mut bytes)
}

fn main() {
    let mut dst: &'static mut [Header] = Box::leak(Box::new([Header { name: "" }; 2]));
    let n = run(&mut dst);
    println!("{n} {:?}", dst);
}
