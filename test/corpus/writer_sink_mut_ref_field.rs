// A derived `Debug` of a struct holding a `&mut i32`, written into a writer
// a piece at a time, and printed: the field's value, read through the
// `&mut`'s cell, as `{:?}` reads it (#1755).
use std::fmt::{self, Write};

struct Log(Vec<String>);

impl Write for Log {
    fn write_str(&mut self, s: &str) -> fmt::Result {
        self.0.push(s.to_string());
        Ok(())
    }
}

#[derive(Debug)]
pub struct S<'a> {
    pub r: &'a mut i32,
}

#[derive(Debug)]
struct Pair<'a>(&'a mut u8, &'a mut String);

fn main() {
    let mut log = Log(Vec::new());
    let mut n = 4;
    write!(log, "{:?}", S { r: &mut n }).unwrap();
    let (mut a, mut b) = (7, String::from("s"));
    write!(log, "|{:?}|{:#?}", Pair(&mut a, &mut b), S { r: &mut n }).unwrap();
    println!("{:?}", log.0);
    let mut u = 5;
    let held = S { r: &mut u };
    println!("{held:?}");
}
