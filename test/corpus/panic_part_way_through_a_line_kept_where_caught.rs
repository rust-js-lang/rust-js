// A writer that panics part-way through a `println!`'s line, in a program
// that catches the panic, leaves what it wrote in stdout's line, as Rust's
// `LineWriter` keeps it, and what's printed after ends that line: `x half`
// then `caught true` (#1496, the user's decision). One that panics before
// writing, one inside another's `write!`, one of a `Debug`, a `print!`'s,
// and one that writes through a `format!` it makes, each caught.
use std::fmt;
use std::panic;

struct Halfway;

impl fmt::Display for Halfway {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "half")?;
        panic!("in the middle");
    }
}

struct Early(Vec<u32>);

impl fmt::Display for Early {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let first = self.0[3];
        write!(f, "{first}")
    }
}

struct Outer;

impl fmt::Display for Outer {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "<{}>", Halfway)
    }
}

struct Shown(u32);

impl fmt::Debug for Shown {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "Shown({}", self.0)?;
        if self.0 > 1 {
            panic!("too big");
        }
        write!(f, ")")
    }
}

struct Made;

impl fmt::Display for Made {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let text = format!("made {}", 7);
        f.write_str(&text)?;
        panic!("after made");
    }
}

fn main() {
    let caught = panic::catch_unwind(|| println!("x {} y", Halfway));
    println!("caught {}", caught.is_err());
    let caught = panic::catch_unwind(|| println!("a {} b", Early(vec![1])));
    println!("early {}", caught.is_err());
    let caught = panic::catch_unwind(|| println!("o {} p", Outer));
    println!("outer {}", caught.is_err());
    println!("{:?}", Shown(1));
    let caught = panic::catch_unwind(|| println!("{:?} {:?}", Shown(1), Shown(2)));
    println!("shown {}", caught.is_err());
    let caught = panic::catch_unwind(|| print!("p {} q", Halfway));
    println!("print {}", caught.is_err());
    let caught = panic::catch_unwind(|| println!("m {}", Made));
    println!("made {}", caught.is_err());
    let text = panic::catch_unwind(|| Halfway.to_string());
    println!("to_string {}", text.is_err());
}
