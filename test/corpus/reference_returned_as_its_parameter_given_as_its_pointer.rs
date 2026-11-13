// httparse's shape: `parse_headers_iter_uninit` takes a `Bytes` holding raw
// pointers, so it may let a reference out as one (ADR 1114) and is given its
// `headers`' place's pointer. What it's given is what `deinit_slice_mut`
// returns, its own parameter reborrowed through `&raw mut *s` and a cast that
// keeps the place: the pointer `parse_headers_iter` was given, the same place,
// so the pointer stored in `bytes` is `ptr::eq` to one made of `headers`, and
// what's written through it after the call reaches `dst` (#1513).
use std::mem::MaybeUninit;

#[derive(Clone, Copy, Debug)]
struct Header {
    name: &'static str,
}

struct Bytes {
    cursor: *const u8,
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
    let taken = std::mem::take(headers);
    *headers = &mut taken[..1];
    unsafe { bytes.cursor = bytes.cursor.add(1) };
    1
}

fn parse_headers_iter(headers: &mut &'static mut [Header], bytes: &mut Bytes) -> usize {
    let at = &raw const *headers;
    let n = parse_headers_iter_uninit(unsafe { deinit_slice_mut(headers) }, bytes);
    println!("same place: {}", std::ptr::eq(at as *const (), bytes.seen as *const ()));
    headers[0].name = "Written after";
    n
}

fn main() {
    let storage: &'static mut [Header] = Box::leak(Box::new([Header { name: "" }; 4]));
    let text = b"Host: x";
    let mut dst: &'static mut [Header] = storage;
    let mut bytes = Bytes { cursor: text.as_ptr(), seen: std::ptr::null() };
    let n = parse_headers_iter(&mut dst, &mut bytes);
    println!("{n} {:?} {}", dst, unsafe { *bytes.cursor });
}
