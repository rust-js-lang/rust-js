// Generic code that writes to any `fmt::Write`, as chrono formats its dates:
// given a `String`, the `Formatter` a `Display` has, and a writer of the
// crate's own, through helpers that write a `char`, a `&str` and a
// `write!` at a time (ADR 0180).

use std::fmt::{self, Write};

fn write_digit(w: &mut (impl Write + ?Sized), v: u8) -> fmt::Result {
    w.write_char((b'0' + v) as char)
}

fn write_two(w: &mut (impl Write + ?Sized), v: u8) -> fmt::Result {
    write_digit(w, v / 10)?;
    write_digit(w, v % 10)
}

fn write_time<W: Write>(w: &mut W, hours: u8, minutes: u8, label: &str) -> fmt::Result {
    write_two(w, hours)?;
    w.write_char(':')?;
    write_two(w, minutes)?;
    if !label.is_empty() {
        w.write_str(" ")?;
        write!(w, "[{}:{:>3}]", label, hours + minutes)?;
    }
    Ok(())
}

struct Time {
    hours: u8,
    minutes: u8,
}

impl fmt::Display for Time {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        f.write_str("at ")?;
        write_time(f, self.hours, self.minutes, "")?;
        f.write_char('.')
    }
}

// A writer of the crate's own, that counts what it's given.
struct Counter {
    text: String,
    chars: usize,
}

impl Write for Counter {
    fn write_str(&mut self, s: &str) -> fmt::Result {
        self.chars += s.chars().count();
        self.text.push_str(s);
        Ok(())
    }
}

// A writer taken by value, as bitflags' `to_writer` takes one: a
// `Formatter` or a `String` given as a `&mut`, which std's
// `impl Write for &mut W` writes through.
fn to_writer(bits: u8, mut writer: impl Write) -> fmt::Result {
    writer.write_str("bits")?;
    writer.write_char('=')?;
    write!(writer, "{bits:#04x}")
}

struct Flags(u8);

impl fmt::Display for Flags {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        to_writer(self.0, f)
    }
}

fn main() {
    let mut s = String::from("> ");
    write_time(&mut s, 9, 5, "am").unwrap();
    println!("{s}");
    let time = Time { hours: 23, minutes: 59 };
    println!("{time} [{:>12}] {}", time.to_string(), time.to_string().len());
    let mut counter = Counter { text: String::new(), chars: 0 };
    write_time(&mut counter, 7, 30, "c").unwrap();
    println!("{} {}", counter.text, counter.chars);
    let mut alone = String::new();
    write_two(&mut alone, 42).unwrap();
    write_digit(&mut alone, 7).unwrap();
    println!("{alone}");
    let mut bits = String::from("[");
    to_writer(5, &mut bits).unwrap();
    bits.push(']');
    println!("{bits} {} {:>12}|", Flags(10), Flags(255).to_string());
    to_writer(9, &mut counter).unwrap();
    println!("{} {}", counter.text, counter.chars);
}
