//@ library-differs
// An `fmt::Arguments` made a `&dyn Display` and written into a writer a
// piece at a time, as itertools' `format_with` gives one to the closure it
// calls, `f(&format_args!("<{x}>"))` (#1712): the function that writes into
// the sink it claims, armed by the writer that writes the `dyn`; and the
// same formatted plainly. The `dyn Display` written frames only `Display`
// writers, so a derived `Debug` of a range, which isn't written a piece at
// a time, still formats as it did.
use std::fmt::{self, Write};

struct Log(Vec<String>);

impl Write for Log {
    fn write_str(&mut self, s: &str) -> fmt::Result {
        self.0.push(s.to_string());
        Ok(())
    }
}

struct FormatWith<'a, F>(&'a [u32], F);

impl<F> fmt::Display for FormatWith<'_, F>
where
    F: Fn(u32, &mut dyn FnMut(&dyn fmt::Display) -> fmt::Result) -> fmt::Result,
{
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        for (i, &x) in self.0.iter().enumerate() {
            if i > 0 {
                f.write_str("|")?;
            }
            (self.1)(x, &mut |shown: &dyn fmt::Display| shown.fmt(f))?;
        }
        Ok(())
    }
}

#[derive(Debug)]
struct Span {
    range: std::ops::Range<u32>,
}

fn format_with<F>(items: &[u32], f: F) -> FormatWith<'_, F>
where
    F: Fn(u32, &mut dyn FnMut(&dyn fmt::Display) -> fmt::Result) -> fmt::Result,
{
    FormatWith(items, f)
}

fn main() {
    let items = [1, 22, 333];
    let shown = format_with(&items, |x, f| f(&format_args!("<{x}>")));
    let mut log = Log(Vec::new());
    write!(log, "{shown}").unwrap();
    println!("{:?}", log.0);
    println!("{shown}");
    println!("{}", format_with(&items, |x, f| f(&x)));
    println!("{:?}", Span { range: 2..5 });
}
