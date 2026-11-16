// A range shown into a writer through generic code, a `T: Debug` written
// into a `W: fmt::Write`: std's `Debug` of a range writes straight to the
// formatter, its start, its dots and its end, each a call of the writer's,
// the bounds by the same options (#1897). The writer records each call.

use std::fmt::{self, Write};
use std::ops::Range;

struct Calls {
    log: Vec<String>,
}

impl Write for Calls {
    fn write_str(&mut self, s: &str) -> fmt::Result {
        self.log.push(format!("str {s:?}"));
        Ok(())
    }

    fn write_char(&mut self, c: char) -> fmt::Result {
        self.log.push(format!("char {c:?}"));
        Ok(())
    }
}

fn show<W: Write, T: fmt::Debug>(w: &mut W, t: T) -> fmt::Result {
    write!(w, "<{t:?}|{t:>4?}>")
}

#[derive(Debug)]
struct Span {
    at: Range<u8>,
    name: &'static str,
}

fn main() {
    let mut calls = Calls { log: Vec::new() };
    show(&mut calls, 1..4).unwrap();
    show(&mut calls, 2..).unwrap();
    show(&mut calls, ..=7).unwrap();
    show(&mut calls, ..3).unwrap();
    show(&mut calls, ..).unwrap();
    show(&mut calls, Span { at: 5..9, name: "x" }).unwrap();
    for line in &calls.log {
        println!("{line}");
    }
    let mut text = String::new();
    show(&mut text, -3i64..3).unwrap();
    show(&mut text, 'a'..'c').unwrap();
    println!("{text}");
}
