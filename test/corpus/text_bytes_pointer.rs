// Text's `as_ptr()`, only read through, a copy's source: a pointer to a
// copy of its UTF-8 bytes, which reads the same, as arrayvec's
// `ArrayString::try_push_str` copies them in; and its bytes back as text,
// `from_utf8_unchecked` of `from_raw_parts`, its `as_str` (ADR 0407).
use std::mem::MaybeUninit;
use std::{ptr, slice, str};

struct Text<const CAP: usize> {
    xs: [MaybeUninit<u8>; CAP],
    len: usize,
}

impl<const CAP: usize> Text<CAP> {
    fn new() -> Self {
        Text { xs: [MaybeUninit::uninit(); CAP], len: 0 }
    }

    fn push_str(&mut self, s: &str) -> bool {
        if s.len() > CAP - self.len {
            return false;
        }
        unsafe {
            let dst = (self.xs.as_mut_ptr() as *mut u8).add(self.len);
            let src = s.as_ptr();
            ptr::copy_nonoverlapping(src, dst, s.len());
        }
        self.len += s.len();
        true
    }

    fn as_str(&self) -> &str {
        unsafe {
            let bytes = slice::from_raw_parts(self.xs.as_ptr() as *const u8, self.len);
            str::from_utf8_unchecked(bytes)
        }
    }
}

fn main() {
    let mut t: Text<8> = Text::new();
    t.push_str("hé");
    t.push_str("llo");
    let full = t.push_str("!!!");
    println!("{} {} {}", t.as_str(), t.len, full);
}
