// The address of a slice made from raw parts is where its items are in the
// array they're of: httparse counts what it parsed as the difference of two
// such slices' `as_ptr() as usize`, its `Bytes` a cursor between two
// pointers, the empty one at the end too (#1424).
unsafe fn slice_from_ptr_range<'a>(start: *const u8, end: *const u8) -> &'a [u8] {
    unsafe { core::slice::from_raw_parts(start, end as usize - start as usize) }
}

struct Bytes<'a> {
    start: *const u8,
    end: *const u8,
    cursor: *const u8,
    text: std::marker::PhantomData<&'a [u8]>,
}

impl<'a> Bytes<'a> {
    fn new(slice: &'a [u8]) -> Bytes<'a> {
        let start = slice.as_ptr();
        let end = unsafe { start.add(slice.len()) };
        Bytes { start, end, cursor: start, text: std::marker::PhantomData }
    }

    fn rest(&self) -> &'a [u8] {
        unsafe { slice_from_ptr_range(self.cursor, self.end) }
    }

    fn skip_while(&mut self, byte: u8) {
        while self.cursor < self.end && unsafe { *self.cursor } != byte {
            self.cursor = unsafe { self.cursor.add(1) };
        }
    }

    fn pos(&self) -> usize {
        self.cursor as usize - self.start as usize
    }
}

fn main() {
    let text: &[u8] = b"abcdef";
    let start = text.as_ptr();
    let end = unsafe { start.add(text.len()) };
    let mid = unsafe { start.add(2) };
    let a = unsafe { slice_from_ptr_range(start, end) }.as_ptr() as usize;
    let b = unsafe { slice_from_ptr_range(mid, end) }.as_ptr() as usize;
    let c = unsafe { slice_from_ptr_range(end, end) }.as_ptr() as usize;
    println!("{} {} {}", b - a, c - a, unsafe { slice_from_ptr_range(mid, end) }.len());

    let request: &[u8] = b"GET /path HTTP/1.1\r\n";
    let mut bytes = Bytes::new(request);
    let begun = bytes.rest().as_ptr() as usize;
    bytes.skip_while(b' ');
    let method = bytes.rest().as_ptr() as usize - begun;
    bytes.skip_while(b'\n');
    let line = bytes.rest().as_ptr() as usize - begun;
    println!("{method} {line} {} {:?}", bytes.pos(), bytes.rest());
}
