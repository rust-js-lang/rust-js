// Guarded arms that fall through (ADR 0905), in a loop whose turn goes on
// past the `match`, an arm of which breaks or continues the loop: the `if`
// chain the arms are, as Rust tries them, no label (#389b, #389c). A subject
// of a `for`'s item, a nested `match` whose arm breaks the loop, an arm whose
// statements branch, one whose `if` goes on to what follows it, and one
// that continues the loop.
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
            b'e' => {
                if n > 2 {
                    n += 3;
                }
                n += 4;
            }
            b'd' => match n {
                0 => break,
                1..10 => n += 100,
                _ => {}
            },
            _ => {}
        }
        i += 1;
        n += 1;
    }
    (i, n)
}

fn main() {
    let log = RefCell::new(Vec::new());
    for text in ["", "a", "aab", "bbbbbbc", "cccccccca", "d!a", "dxd", "abcdabcd", "ccccabcd!", "eee", "aeae"] {
        let scanned = scan(text.as_bytes(), &log);
        println!("{text:?}: {scanned:?} {:?}", log.take());
    }
}
