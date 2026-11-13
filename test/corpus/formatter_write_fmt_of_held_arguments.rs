// A `Formatter`'s `write_fmt` of an `Arguments` it didn't make in place
// writes its text: one held in a variable, a parameter's, a constant's, a
// literal's or one made with arguments, as `write!` writes its own
// (#1722).
use std::fmt;

const GREETING: fmt::Arguments<'static> = format_args!("hello");

fn prefixed(f: &mut fmt::Formatter<'_>, args: fmt::Arguments<'_>) -> fmt::Result {
    f.write_str("~")?;
    f.write_fmt(args)
}

struct Shown(i32);

impl fmt::Display for Shown {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        prefixed(f, format_args!("lit"))?;
        prefixed(f, format_args!("n={:>3}", self.0))?;
        prefixed(f, GREETING)?;
        let held = format_args!("<{}>", self.0);
        f.write_fmt(held)?;
        f.write_fmt(held)?;
        f.write_fmt(GREETING)?;
        fmt::Display::fmt(&GREETING, f)
    }
}

fn main() {
    println!("{} {}", Shown(3), GREETING);
    let s = format!("{:>30}|{}", Shown(-1), Shown(2).to_string().len());
    println!("{s}");
}
