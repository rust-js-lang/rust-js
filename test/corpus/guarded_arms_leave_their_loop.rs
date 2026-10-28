// Guarded arms that fall through (ADR 0905), in a loop whose turn goes on
// past the `match`, an arm of which breaks or continues the loop: the `if`
// chain the arms are, as Rust tries them, no label (#389b),
// each guard run only where Rust runs it.
use std::cell::RefCell;

fn on(log: &RefCell<Vec<String>>, name: &str, holds: bool) -> bool {
    log.borrow_mut().push(format!("{name}:{holds}"));
    holds
}

fn scan(codes: &[u8], log: &RefCell<Vec<String>>) -> (usize, u32) {
    let mut i = 0;
    let mut n = 0;
    for &code in codes {
        match code {
            b'a' if on(log, "a", n > 3) => break,
            b'a' | b'b' if on(log, "ab", n % 2 == 0) => {
                i += 2;
                continue;
            }
            b'a' | b'b' | b'c' => {
                if n > 5 {
                    n += 10;
                } else {
                    n += 1;
                }
            }
            b'd' if on(log, "d", n == 0) => break,
            b'd' | b'e' => n += 100,
            _ => {}
        }
        i += 1;
        n += 1;
    }
    (i, n)
}

fn polled(codes: &[u8], log: &RefCell<Vec<String>>) -> u32 {
    let mut at = 0;
    let mut n = 0;
    let mut next = || {
        at += 1;
        codes.get(at - 1).copied().unwrap_or(0)
    };
    loop {
        match next() {
            0 => break,
            1 if on(log, "one", n > 2) => break,
            1 | 2 => {
                if n == 7 {
                    n += 5;
                    continue;
                }
                n += 1;
            }
            _ => n += 10,
        }
        n += 1;
    }
    n
}

fn main() {
    let log = RefCell::new(Vec::new());
    for text in ["", "a", "aab", "bbbbbbc", "cccccccca", "d!a", "dxd", "abcdabcd", "ccccabcd!"] {
        let scanned = scan(text.as_bytes(), &log);
        println!("{text:?}: {scanned:?} {:?}", log.take());
        let polled = polled(text.as_bytes(), &log);
        println!("{polled} {:?}", log.take());
    }
    for codes in [&[1u8, 1, 1, 1, 1][..], &[2, 2, 2, 2, 2, 1, 3], &[3, 2, 1, 1, 0]] {
        let polled = polled(codes, &log);
        println!("{codes:?}: {polled} {:?}", log.take());
    }
}
