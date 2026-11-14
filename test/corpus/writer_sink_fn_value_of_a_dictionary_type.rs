//@ library-differs
// A std formatting trait's `fmt` taken as a `fn` pointer of a type only a
// dictionary knows, as itertools' `Format` gives `fmt::Display::fmt` of its
// `I::Item` (#1712): the function that writes into the sink it claims by
// that dictionary, `$writeBy`, so a framed writer that calls it arms it;
// called plainly, its text.
use std::fmt::{self, Write};

struct Log(Vec<String>);

impl Write for Log {
    fn write_str(&mut self, s: &str) -> fmt::Result {
        self.0.push(s.to_string());
        Ok(())
    }
}

struct Format<'a, T>(&'a [T]);

impl<T> Format<'_, T> {
    fn format(&self, f: &mut fmt::Formatter<'_>, each: fn(&T, &mut fmt::Formatter<'_>) -> fmt::Result) -> fmt::Result {
        for (i, item) in self.0.iter().enumerate() {
            if i > 0 {
                f.write_str(", ")?;
            }
            each(item, f)?;
        }
        Ok(())
    }
}

impl<T: fmt::Display> fmt::Display for Format<'_, T> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        self.format(f, fmt::Display::fmt)
    }
}

impl<T: fmt::Debug> fmt::Debug for Format<'_, T> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        self.format(f, fmt::Debug::fmt)
    }
}

fn main() {
    let mut log = Log(Vec::new());
    write!(log, "[{}]", Format(&[1, 20])).unwrap();
    write!(log, "[{:?}]", Format(&["a", "b"])).unwrap();
    println!("{:?}", log.0);
    println!("{} {:?}", Format(&[1.5, 2.0]), Format(&['x']));
}
