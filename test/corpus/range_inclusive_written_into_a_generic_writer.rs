// A `RangeInclusive` shown into a writer through generic code, a `T: Debug`
// written into a `W: fmt::Write`: its start, its dots and its end, each a
// call of the writer's, then ` (exhausted)` only where it's used up, tested
// after the end is written, as std's `fmt` does (#1963). Of numbers and
// `char`s, used up and not, with options; a writer that records each call;
// and one that fails at a call, the third, then the last, ` (exhausted)`
// itself, whose error stops the rest.

use std::fmt::{self, Write};
use std::ops::RangeInclusive;

struct Calls {
    log: Vec<String>,
    fail_at: usize,
}

impl Write for Calls {
    fn write_str(&mut self, s: &str) -> fmt::Result {
        if self.log.len() + 1 == self.fail_at {
            return Err(fmt::Error);
        }
        self.log.push(format!("str {s:?}"));
        Ok(())
    }

    fn write_char(&mut self, c: char) -> fmt::Result {
        if self.log.len() + 1 == self.fail_at {
            return Err(fmt::Error);
        }
        self.log.push(format!("char {c:?}"));
        Ok(())
    }
}

fn show<W: Write, T: fmt::Debug>(w: &mut W, t: &T) -> fmt::Result {
    write!(w, "{t:?}")
}

fn shown_wide<W: Write, T: fmt::Debug>(w: &mut W, t: &T) -> fmt::Result {
    write!(w, "<{t:>3?}>")
}

fn used_up(mut range: RangeInclusive<u8>) -> RangeInclusive<u8> {
    for _ in range.by_ref() {}
    range
}

fn main() {
    let done = used_up(1..=3);
    let fresh = 5u8..=7;
    let backwards = 9u8..=2;
    let mut letters = 'a'..='c';
    letters.by_ref().for_each(drop);
    let mut half = 'x'..='z';
    half.next();
    let mut one = 3u8..=3;
    one.next();
    let mut top = 255u8..=255;
    top.next();

    let mut text = String::new();
    for range in [&done, &fresh, &backwards, &one, &top] {
        show(&mut text, range).unwrap();
        text.push(' ');
        shown_wide(&mut text, range).unwrap();
        text.push(' ');
    }
    show(&mut text, &letters).unwrap();
    text.push(' ');
    show(&mut text, &half).unwrap();
    println!("{text}");

    // Its eight writes, the last ` (exhausted)`: failing at the third, and
    // at the eighth, inside the test.
    for fail_at in [0, 3, 8] {
        let mut calls = Calls { log: Vec::new(), fail_at };
        let result = show(&mut calls, &letters);
        println!("{fail_at}: {result:?} {:?}", calls.log);
    }
    let mut calls = Calls { log: Vec::new(), fail_at: 0 };
    shown_wide(&mut calls, &letters).unwrap();
    show(&mut calls, &fresh).unwrap();
    println!("{:?}", calls.log);
}
