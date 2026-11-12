// A writer that keeps the result of a `write!` into its `Formatter` of what
// may fail, `let r = write!(f, "{:?}", inner)`, gets the `Err` and writes on,
// what the inner writer wrote before its error kept in the line; or drops
// it, `let _ = write!(..)`, or asks it at once, `.is_err()`. By `?`, or
// as its own last result, its error is the outer writer's own (ADR 0187,
// #1524). Of a writer that prints as it formats too (#1490), its print
// where it ran.
use std::fmt;

struct D(u32);

impl fmt::Debug for D {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "D({})", self.0)?;
        if self.0 == 3 {
            return Err(fmt::Error);
        }
        Ok(())
    }
}

struct Kept(D);

impl fmt::Display for Kept {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "a[")?;
        let r = write!(f, "{:?}", self.0);
        write!(f, "<{}>]", r.is_err())
    }
}

struct Dropped(D);

impl fmt::Display for Dropped {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let _ = write!(f, "{:?} ", self.0);
        let asked = write!(f, "{:?}", self.0).is_err();
        write!(f, " {asked}")
    }
}

struct Passed(D);

impl fmt::Display for Passed {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "p[")?;
        write!(f, "{:?}", self.0)?;
        write!(f, "]")
    }
}

/// Its last write its own result.
struct Last(D);

impl fmt::Display for Last {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "l ")?;
        write!(f, "{:?}", self.0)
    }
}

/// Prints as it formats, and fails at 3.
struct Loud(u32);

impl fmt::Debug for Loud {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        print!("!");
        write!(f, "L({})", self.0)?;
        if self.0 == 3 {
            return Err(fmt::Error);
        }
        Ok(())
    }
}

struct KeptLoud(Loud);

impl fmt::Display for KeptLoud {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "k[")?;
        let r = write!(f, "{:?}", self.0);
        print!("?");
        write!(f, "<{}>]", r.is_err())
    }
}

fn main() {
    println!("z {}", Kept(D(3)));
    println!("y {}", Kept(D(2)));
    println!("{}", Kept(D(3)).to_string());
    println!("{}", Dropped(D(3)));
    println!("{}", Dropped(D(4)));
    println!("{}", Passed(D(5)));
    println!("{:?}", format!("{}", Passed(D(4))));
    println!("{}", Last(D(6)));
    println!("{}", Kept(D(3)).to_string() + &format!("{}", Last(D(7))));
    println!("x {} y", KeptLoud(Loud(3)));
    println!("x {} y", KeptLoud(Loud(1)));
    let s = format!("{}", KeptLoud(Loud(3)));
    println!("{s}");
}
