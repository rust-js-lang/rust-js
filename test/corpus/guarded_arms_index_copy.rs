// Guarded arms that fall through, in a loop, of a slice's item at an
// index: the `if` chain reads `codes[i]` as written, not MIR's copy of the
// index, `const value = i;`, as no test can set `i` (#389c). An index a
// thread-local holds, which a guard's call sets, is read through the copy
// still: each test reads the item the `match` matched.
use std::cell::Cell;

thread_local! {
    static AT: Cell<usize> = const { Cell::new(0) };
}

fn on(n: u32) -> bool {
    n % 3 == 0
}

fn scan(codes: &[u8]) -> (usize, u32) {
    let mut i = 0;
    let mut n = 0;
    while i < codes.len() {
        match codes[i] {
            b'a' if on(n) => break,
            b'a' | b'b' if on(n + 1) => {
                i += 2;
                continue;
            }
            b'a' | b'b' | b'c' => n += 3,
            _ => {}
        }
        i += 1;
        n += 1;
    }
    (i, n)
}

fn step(n: u32) -> bool {
    AT.set(AT.get() + 1);
    on(n)
}

fn scan_held(codes: &[u8]) -> u32 {
    AT.set(0);
    let mut n = 0;
    while AT.get() < codes.len() {
        match codes[AT.get()] {
            b'a' if step(n) => break,
            b'a' | b'b' if on(n + 1) => {
                n += 5;
                continue;
            }
            b'a' | b'b' | b'c' => n += 3,
            _ => {}
        }
        AT.set(AT.get() + 1);
        n += 1;
    }
    n
}

fn main() {
    for text in ["", "a", "ab", "bbbc", "cabca", "xaxbxc", "aaaaaa", "bacbacbac"] {
        let scanned = scan(text.as_bytes());
        let held = scan_held(text.as_bytes());
        println!("{text:?}: {scanned:?} {held}");
    }
}
