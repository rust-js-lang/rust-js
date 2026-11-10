// A pointer into a text's bytes, `s.as_bytes().as_ptr()` or `s.as_ptr()`,
// stepped along them, read through and compared with another into the same
// bytes, as a byte-at-a-time search does (ADR 0126, amended, #1183): each
// reads Rust's bytes and compares as Rust's does, of a short text and of a
// long one, whose bytes JS keeps.
fn commas(s: &str) -> usize {
    let start = s.as_bytes().as_ptr();
    let end = unsafe { start.add(s.len()) };
    let mut p = start;
    let mut commas = 0;
    while p < end {
        if unsafe { *p } == b',' {
            commas += 1;
        }
        p = unsafe { p.add(1) };
    }
    commas
}

fn last_space(s: &str) -> Option<usize> {
    let start = s.as_ptr();
    let mut p = unsafe { start.add(s.len()) };
    while p > start {
        p = unsafe { p.sub(1) };
        if unsafe { p.read() } == b' ' {
            return Some(unsafe { p.offset_from(start) } as usize);
        }
    }
    None
}

fn wide(s: &str, at: usize) -> u16 {
    let bytes = s.as_bytes();
    unsafe { bytes.as_ptr().add(at).cast::<u16>().read_unaligned() }
}

fn main() {
    let short = String::from("a, b, é");
    let long = "héllo, wörld ".repeat(20);
    for s in [short.as_str(), long.as_str()] {
        println!("{} {:?} {:#x} {}", commas(s), last_space(s), wide(s, 1), unsafe { s.as_ptr().add(2).read() });
    }
}
