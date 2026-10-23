// A format string written as a `concat!` of literals is its pieces joined
// by `+`, each a template where it holds a placeholder and otherwise a
// string (ADR 0641), as React's formatProdErrorMessage writes its message;
// one whose placeholder or `{{` spans pieces, or whose value rustc writes
// in the template, is one template.

//@ run-fail: bad 3

use std::fmt;
use std::fmt::Write;

fn react(code: u32, url: &str) -> String {
    format!(
        concat!(
            "Minified React error #{}; visit {} for the full message or ",
            "use the non-minified dev environment for full errors and additional ",
            "helpful warnings."
        ),
        code,
        url
    )
}

struct Point {
    x: i32,
    y: i32,
}

impl fmt::Display for Point {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, concat!("(", "{}, {}", ")"), self.x, self.y)
    }
}

struct Counter {
    n: u32,
}

impl Counter {
    fn bump(&mut self) -> Option<u32> {
        self.n += 1;
        Some(self.n)
    }
}

fn pair(n: u32) -> (u32, u32) {
    (n, n + 1)
}

const N: u32 = 3;

fn show(n: u32, f: f64, a: &str, b: &str) {
    println!(concat!("pairs ", "{:?} and ", "{:?}!"), pair(n), pair(1));
    println!(concat!("n is ", "{}", "!"), N);
    println!(concat!("n + 1 is ", "{}", "!"), N + 1);
    println!(concat!("x is ", "{}", "!"), n);
    println!(concat!("{1} then ", "{0}", " and {1:?}"), a, b);
    println!(concat!("[{:>8.2}] ", "{x:.1$} {n}"), f, n as usize, x = f, n = n);
    println!(concat!("{{", "{}}}", " ", "{:<4}|"), n, a);
    println!(concat!("{{{}}} ", "{:<4}|"), n, a);
    println!(concat!("<{", "}>"), n);
    println!(concat!("{} and ", "{}"), "lit", n);
    println!(concat!("<a>\n{}", "</a>"), n);
    println!(concat!("x is ", r"{}
", 1, '!'), n);
}

fn main() {
    println!("{}", react(31, "https://react.dev/errors/31"));
    println!(concat!("no ", "placeholder"));
    show(7, 3.14159, "a", "b");
    println!(concat!("a{", "{b"));
    println!(concat!("p is ", "{}"), Point { x: 1, y: -2 });
    let mut c = Counter { n: 0 };
    println!(concat!("{:?} ", "{:?}"), c.bump(), c.bump());
    let mut s = String::new();
    let (one, two) = (c.n, c.n + 1);
    write!(s, concat!("w ", "{}"), one).unwrap();
    writeln!(s, concat!(" l ", "{}"), two).unwrap();
    print!("{s}");
    print!(concat!("e ", "{}\n", "tail\n"), one);
    panic!(concat!("bad ", "{}"), two);
}
