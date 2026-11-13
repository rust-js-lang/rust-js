//@ compile-fail: a raw pointer to what a `&mut` a call lends refers to
// A function that returns its parameter on one path and another reference
// on the other: what a call of it returns is no pointer its caller was
// given, so a raw pointer to it, `parse_headers_iter_uninit`'s, given its
// place's pointer (ADR 1114), is refused, as any call's cell is (#1513).
use std::mem::MaybeUninit;

#[derive(Clone, Copy, Debug)]
struct Header {
    name: &'static str,
}

struct Bytes {
    cursor: *const u8,
    seen: *const &'static mut [MaybeUninit<Header>],
}

unsafe fn deinit_slice_mut<'a, 'b, T>(
    s: &'a mut &'b mut [T],
    other: &'a mut &'b mut [MaybeUninit<T>],
    first: bool,
) -> &'a mut &'b mut [MaybeUninit<T>] {
    if first {
        let s: *mut &mut [T] = s;
        let s = s as *mut &mut [MaybeUninit<T>];
        unsafe { &mut *s }
    } else {
        other
    }
}

fn parse_headers_iter_uninit(headers: &mut &'static mut [MaybeUninit<Header>], bytes: &mut Bytes) -> usize {
    bytes.seen = &raw const *headers;
    headers[0] = MaybeUninit::new(Header { name: "Host" });
    let taken = std::mem::take(headers);
    *headers = &mut taken[..1];
    unsafe { bytes.cursor = bytes.cursor.add(1) };
    1
}

fn parse_headers_iter(
    headers: &mut &'static mut [Header],
    other: &mut &'static mut [MaybeUninit<Header>],
    bytes: &mut Bytes,
) -> usize {
    let at = &raw const *headers;
    let n = parse_headers_iter_uninit(unsafe { deinit_slice_mut(headers, other, true) }, bytes);
    println!("same place: {}", std::ptr::eq(at as *const (), bytes.seen as *const ()));
    n
}

fn main() {
    let storage: &'static mut [Header] = Box::leak(Box::new([Header { name: "" }; 4]));
    let text = b"Host: x";
    let mut dst: &'static mut [Header] = storage;
    let mut bytes = Bytes { cursor: text.as_ptr(), seen: std::ptr::null() };
    let mut spare: &'static mut [MaybeUninit<Header>] = Box::leak(Box::new([MaybeUninit::uninit(); 2]));
    let n = parse_headers_iter(&mut dst, &mut spare, &mut bytes);
    println!("{n} {:?} {}", dst, unsafe { *bytes.cursor });
}
