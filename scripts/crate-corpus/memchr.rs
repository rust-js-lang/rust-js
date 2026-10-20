// memchr's searches (ADR 0529): each finds a byte, one of two or one of
// three, first or last, or each of them from either end, its index in the
// slice it's given, as memchr does, on empty slices, at both ends, in a
// slice of a slice, of every byte value, of repeated and equal needles.
pub fn report() -> String {
    let mut out = String::new();
    let text = b"hello world, hello rust";
    let bytes: Vec<u8> = (0..=255u8).collect();
    let empty: &[u8] = &[];
    out += &format!(
        "{:?} {:?} {:?}\n",
        memchr::memchr(b'o', text),
        memchr::memrchr(b'o', text),
        memchr::memchr(b'z', text)
    );
    out += &format!("{:?} {:?}\n", memchr::memchr(b'h', text), memchr::memrchr(b't', text));
    out += &format!("{:?} {:?}\n", memchr::memchr(b'a', empty), memchr::memrchr(b'a', empty));
    out += &format!(
        "{:?} {:?}\n",
        memchr::memchr(b'o', &text[5..]),
        memchr::memrchr(b'h', &text[1..13])
    );
    out += &format!(
        "{:?} {:?}\n",
        memchr::memchr2(b'w', b'r', text),
        memchr::memrchr2(b'w', b'e', text)
    );
    out += &format!(
        "{:?} {:?}\n",
        memchr::memchr3(b'x', b'y', b',', text),
        memchr::memrchr3(b'l', b'l', b'l', text)
    );
    out += &format!(
        "{:?} {:?}\n",
        memchr::memchr2(b'q', b'q', text),
        memchr::memchr3(b'q', b'v', b'z', text)
    );
    let found: Vec<Option<usize>> = (0..=255u8).step_by(51).map(|b| memchr::memchr(b, &bytes)).collect();
    out += &format!("{found:?} {:?}\n", memchr::memrchr(255, &bytes));
    let all: Vec<usize> = memchr::memchr_iter(b'l', text).collect();
    let back: Vec<usize> = memchr::memrchr_iter(b'l', text).collect();
    out += &format!("{all:?} {back:?}\n");
    let mut it = memchr::memchr2_iter(b'o', b'r', text);
    let first = it.next();
    let last = it.next_back();
    let copy = it.clone();
    let rest: Vec<usize> = it.collect();
    let copied: Vec<usize> = copy.rev().collect();
    out += &format!("{first:?} {last:?} {rest:?} {copied:?}\n");
    let mut three = memchr::memchr3_iter(b'h', b'e', b'!', text);
    let firsts = (three.next(), three.next(), three.next(), three.next(), three.next());
    out += &format!("{firsts:?} {:?}\n", three.next());
    let none: Vec<usize> = memchr::memchr_iter(b'a', empty).collect();
    out += &format!("{none:?} {}\n", memchr::memchr_iter(b'l', &text[3..]).count());
    let every = (0..=255u8)
        .filter(|&b| memchr::memchr(b, &bytes) == Some(b as usize) && memchr::memrchr(b, &bytes) == Some(b as usize))
        .count();
    let adjacent: Vec<usize> = memchr::memchr_iter(b'a', b"aaba").collect();
    let adjacent_back: Vec<usize> = memchr::memrchr2_iter(b'a', b'b', b"aaba").collect();
    let mut one = memchr::memchr_iter(b'a', b"aXa");
    let met = (one.next_back(), one.next(), one.next(), one.next_back());
    out += &format!(
        "{every} {adjacent:?} {adjacent_back:?} {met:?} {:?}\n",
        memchr::memrchr2(b'h', b'z', b"hello")
    );
    let generated: Vec<u8> = (0..4000u32).map(|i| (i * 37 % 251) as u8).collect();
    for needles in [[0u8, 0, 0], [7, 250, 7], [1, 2, 3]] {
        let forward: usize = memchr::memchr3_iter(needles[0], needles[1], needles[2], &generated).sum();
        let backward: usize = memchr::memrchr3_iter(needles[0], needles[1], needles[2], &generated).sum();
        let mut both = memchr::memchr2_iter(needles[0], needles[1], &generated[100..]);
        let mixed = (both.next(), both.next_back(), both.next(), both.next_back());
        out += &format!(
            "{forward} {backward} {mixed:?} {:?}\n",
            memchr::memrchr(needles[2], &generated[..3000])
        );
    }
    out
}
