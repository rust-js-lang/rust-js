// A writer called through a `fn` pointer it's given with its `Formatter`,
// `cb(x, f)`, as itertools' `Format` calls `Display::fmt` or `Debug::fmt`:
// what it writes, the `Formatter`'s options its own; std's trait method as
// such a value, the function its dictionary has (ADR 0538).
use std::fmt;

struct Joined<'a>(&'a [u32], &'a str);

impl Joined<'_> {
    fn format(&self, f: &mut fmt::Formatter, cb: fn(&u32, &mut fmt::Formatter) -> fmt::Result) -> fmt::Result {
        for (i, x) in self.0.iter().enumerate() {
            if i > 0 {
                f.write_str(self.1)?;
            }
            cb(x, f)?;
        }
        Ok(())
    }
}

fn quoted(x: &u32, f: &mut fmt::Formatter) -> fmt::Result {
    write!(f, "'{x}'")
}

impl fmt::Display for Joined<'_> {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        self.format(f, fmt::Display::fmt)
    }
}

impl fmt::LowerHex for Joined<'_> {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        self.format(f, fmt::LowerHex::fmt)
    }
}

impl fmt::Debug for Joined<'_> {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        self.format(f, quoted)
    }
}

fn main() {
    let j = Joined(&[10, 255], ", ");
    println!("{j} {j:x} {j:#x} [{j:>4}] {j:?}");
    println!("[{}]", Joined(&[], "-"));
}
