//@ run-fail: ~loud panic at 33
// A writer that prints a whole line as it formats and then panics, part-way
// through a `println!`'s line, in a program that catches no panic: the line
// it printed is written, as Rust's stdout writes a line at its newline; the
// rest of the line is lost (the guide's difference 57). writer_sink_arguments_kept
// made loud by the metamorphic net's `loud-call`, its 33rd event a panic (#1761).
use std::fmt::{self, Display, Write};
struct Log(Vec<String>);
impl Write for Log {
    fn write_str(&mut self, s: &str) -> fmt::Result {
        self.0.push(s.to_string());
        Ok(())
    }
}
struct Failing(u32, u32, String);
impl Write for Failing {
    fn write_str(&mut self, s: &str) -> fmt::Result {
        self.0 += 1;
        if self.0 == self.1 {
            return Err(fmt::Error);
        }
        self.2.push_str(s);
        Ok(())
    }
}
enum Color {
    Red,
    Named(&'static str, u8),
}
impl Display for Color {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Color::Red => Display::fmt(&format_args!("red"), f),
            Color::Named(name, n) => Display::fmt(&format_args!("{name}#{n:>3}"), f),
        }
    }
}
struct Line<'a>(fmt::Arguments<'a>);
impl Display for Line<'_> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str("[")?;
        Display::fmt(&self.0, f)?;
        f.write_str("]")
    }
}
struct Maybe<'a>(Option<fmt::Arguments<'a>>);
impl Display for Maybe<'_> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match &self.0 {
            Some(args) => Display::fmt(args, f),
            None => f.write_str("-"),
        }
    }
}
struct Boxed<'a>(Box<fmt::Arguments<'a>>, fmt::Arguments<'a>);
impl Display for Boxed<'_> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        Display::fmt(&*self.0, f)?;
        f.write_fmt(self.1)
    }
}
fn given(args: fmt::Arguments<'_>) -> fmt::Arguments<'_> {
    args
}
fn shown(args: &fmt::Arguments<'_>, f: &mut fmt::Formatter<'_>) -> fmt::Result {
    f.write_str("<")?;
    Display::fmt(args, f)?;
    f.write_str(">")
}
struct Lent(i32);
impl Display for Lent {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        shown(&format_args!("{}:{}", self.0, -self.0), f)
    }
}
struct Captured(u8);
impl Display for Captured {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let args = format_args!("{}{}", self.0, 'c');
        let show = |f: &mut fmt::Formatter<'_>| {
            metamorph_loud(format_args!("call 1"));
            (Display::fmt(&args, f))
        };
        show(f)?;
        show(f)
    }
}
const GREETING: fmt::Arguments<'static> = format_args!("hello");
struct Greeted;
impl Display for Greeted {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_fmt(GREETING)?;
        Display::fmt(&GREETING, f)
    }
}
struct Relayed;
impl Display for Relayed {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_fmt(given(format_args!("{}-{}", 1, "r")))?;
        Display::fmt(&given(format_args!("{}+{}", 2, 's')), f)
    }
}
fn log_to(w: &mut Log, args: fmt::Arguments<'_>) -> fmt::Result {
    w.write_fmt(args)
}
fn said(what: fmt::Arguments<'_>) -> usize {
    what.to_string().len()
}
fn logged<T: Display>(value: &T) -> String {
    let mut log = Log(Vec::new());
    write!(log, "{value}").unwrap();
    format!("{:?}", log.0)
}
fn failing<T: Display>(value: &T, at: u32) -> String {
    let mut w = Failing(0, at, String::new());
    let r = write!(w, "{value}");
    format!("{:?} {:?}", r, w.2)
}
fn main() {
    let colors = [Color::Red, Color::Named("teal", 7)];
    for color in &colors {
        metamorph_loud(format_args!("step 1"));
        println!("{color} {:>10}| {}", color, logged(color));
    }
    println!("{}", failing(&colors[1], 2));
    let line = Line(format_args!("{}|{:^5}|{}", 1, 'x', "end"));
    println!("{line} {}", logged(&line));
    for at in 1..=6 {
        metamorph_loud(format_args!("step 2"));
        println!("{}", failing(&line, at));
    }
    let some = Maybe(Some(format_args!("{}{}", 'a', 2)));
    println!("{some} {} {}", logged(&some), logged(&Maybe(None)));
    println!("{}", failing(&some, 2));
    let boxed = Boxed(Box::new(format_args!("boxed")), format_args!("{}{}", 'w', 2));
    println!("{boxed} {}", logged(&boxed));
    println!("{}", failing(&boxed, 3));
    println!("{} {}", Lent(4), logged(&Lent(4)));
    println!("{}", failing(&Lent(4), 3));
    println!("{} {}", Captured(9), logged(&Captured(9)));
    println!("{}", failing(&Captured(9), 3));
    println!("{} {}", Relayed, logged(&Relayed));
    for at in [2, 5] {
        metamorph_loud(format_args!("step 3"));
        println!("{}", failing(&Relayed, at));
    }
    println!("{} {} {GREETING}", Greeted, logged(&Greeted));
    println!("{}", failing(&Greeted, 2));
    let mut log = Log(Vec::new());
    log_to(&mut log, given(format_args!("{}|{:>3}", 1, 'k'))).unwrap();
    println!("{:?}", log.0);
    println!("{}", said(format_args!("{}-{}", 12, 'z')));
}
impl Drop for Log {
    fn drop(&mut self) {
        metamorph_loud(format_args!("drop Log"));
    }
}
impl Drop for Failing {
    fn drop(&mut self) {
        metamorph_loud(format_args!("drop Failing"));
    }
}
impl Drop for Color {
    fn drop(&mut self) {
        metamorph_loud(format_args!("drop Color"));
    }
}
impl<'a> Drop for Line<'a> {
    fn drop(&mut self) {
        metamorph_loud(format_args!("drop Line"));
    }
}
impl<'a> Drop for Maybe<'a> {
    fn drop(&mut self) {
        metamorph_loud(format_args!("drop Maybe"));
    }
}
impl<'a> Drop for Boxed<'a> {
    fn drop(&mut self) {
        metamorph_loud(format_args!("drop Boxed"));
    }
}
impl Drop for Lent {
    fn drop(&mut self) {
        metamorph_loud(format_args!("drop Lent"));
    }
}
impl Drop for Captured {
    fn drop(&mut self) {
        metamorph_loud(format_args!("drop Captured"));
    }
}
impl Drop for Greeted {
    fn drop(&mut self) {
        metamorph_loud(format_args!("drop Greeted"));
    }
}
impl Drop for Relayed {
    fn drop(&mut self) {
        metamorph_loud(format_args!("drop Relayed"));
    }
}
static METAMORPH_LOUD: ::std::sync::atomic::AtomicUsize = ::std::sync::atomic::AtomicUsize::new(0);
const METAMORPH_PANIC_AT: usize = 33usize;
fn metamorph_loud(what: ::std::fmt::Arguments) {
    let n = METAMORPH_LOUD.fetch_add(1, ::std::sync::atomic::Ordering::Relaxed) + 1;
    if n <= 2000usize {
        let said: String = ::std::string::ToString::to_string(&what).chars().take(160).collect();
        println!("~loud {} {}", n, said);
    }
    if n == METAMORPH_PANIC_AT && !::std::thread::panicking() {
        panic!("~loud panic at {}", n);
    }
}
