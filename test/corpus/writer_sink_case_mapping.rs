// A `char`'s case mapping's `Display`, `to_lowercase()`'s, written into a
// writer as core's `CaseMappingIter` writes it, each `char` a `write_char`
// (#1712): directly, by a writer of the crate's own, as heck's `transform`
// writes each word, and by generic code; one mapped to more than one
// `char`, `'İ'`'s, and printed.
use std::fmt::{self, Write};

struct Log(Vec<String>);

impl Write for Log {
    fn write_str(&mut self, s: &str) -> fmt::Result {
        self.0.push(format!("s{s}"));
        Ok(())
    }

    fn write_char(&mut self, c: char) -> fmt::Result {
        self.0.push(format!("c{c}"));
        Ok(())
    }
}

struct Kebab<'a>(&'a str);

impl fmt::Display for Kebab<'_> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        for (i, word) in self.0.split(' ').enumerate() {
            if i > 0 {
                f.write_char('-')?;
            }
            for c in word.chars() {
                write!(f, "{}", c.to_lowercase())?;
            }
        }
        Ok(())
    }
}

fn written<W: Write, T: fmt::Display>(w: &mut W, t: T) {
    write!(w, "<{t}>").unwrap();
}

fn main() {
    let mut log = Log(Vec::new());
    write!(log, "{}|{}", 'A'.to_lowercase(), 'ß'.to_uppercase()).unwrap();
    write!(log, "|{}", 'İ'.to_lowercase()).unwrap();
    write!(log, "|{}", Kebab("Hello Big World")).unwrap();
    written(&mut log, 'Q'.to_lowercase());
    println!("{:?}", log.0);
    println!("{} {} {}", Kebab("A B"), 'x'.to_uppercase(), 'İ'.to_lowercase());
}
