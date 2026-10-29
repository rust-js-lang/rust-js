// A `while` whose way out goes on to a tuple the function returns, `(i,
// n)`, which MIR builds in that way out, of its parts: after the loop, as
// written, `while (i < codes.length) { .. } return [i, n];`, its test the
// `while`'s; and of guarded arms that fall through, with a nested `match`
// whose arm breaks the loop, their `if` chain, no label (#389c).
use std::cell::RefCell;

fn on(log: &RefCell<Vec<String>>, name: &str, holds: bool) -> bool {
    log.borrow_mut().push(format!("{name}:{holds}"));
    holds
}

fn first_one(codes: &[u8]) -> (usize, u32) {
    let mut i = 0;
    let mut n = 0;
    while i < codes.len() {
        if codes[i] == 1 {
            break;
        }
        i += 1;
        n += u32::from(codes[i - 1]);
    }
    (i, n)
}

fn scan(codes: &[u8], log: &RefCell<Vec<String>>) -> (usize, u32) {
    let mut i = 0;
    let mut n = 0;
    while i < codes.len() {
        match codes[i] {
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
    for codes in [&[][..], &[3, 4, 1, 9], &[2, 2, 2]] {
        println!("{codes:?}: {:?}", first_one(codes));
    }
    let log = RefCell::new(Vec::new());
    for text in ["", "a", "aab", "bbbbbbc", "cccccccca", "d!a", "dxd", "abcdabcd", "ccccabcd!"] {
        let scanned = scan(text.as_bytes(), &log);
        println!("{text:?}: {scanned:?} {:?}", log.take());
    }
}
