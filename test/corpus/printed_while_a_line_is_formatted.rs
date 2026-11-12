// What a writer prints as `println!` formats lands inside the line, as
// Rust's stdout, a `LineWriter`, puts it (#1490, ADR 1130): a writer's text
// written before its print comes before it, a drop a writer runs prints
// after the text written before it, a writer formatted inside another's
// write prints between the other's pieces, and one formatted for
// `format!` or `to_string()` inside a writer is made apart from the line.
// A `print!` part-way leaves the line open for what comes next; standard
// error is written at once. A writer that prints and then panics leaves
// what it wrote in the line, which the next line ends. A derived `Debug` of
// such a value is refused (#1495).
use std::fmt;
use std::io::Write;

struct Noisy(u32);

impl fmt::Display for Noisy {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "<")?;
        println!("inside {}", self.0);
        eprintln!("err {}", self.0);
        write!(f, "{}>", self.0)
    }
}

struct Loud(u32);

impl Drop for Loud {
    fn drop(&mut self) {
        println!("drop {}", self.0);
    }
}

impl fmt::Debug for Loud {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "Loud({})", self.0)
    }
}

struct Grid(u32);

impl fmt::Debug for Grid {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let cells: Vec<Loud> = (0..self.0).map(Loud).collect();
        f.debug_list().entries(cells.iter()).finish()
    }
}

struct Outer;

impl fmt::Display for Outer {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "x")?;
        write!(f, "{}", Noisy(7))?;
        let made = format!("{}", Noisy(8));
        let text = Noisy(9).to_string();
        write!(f, "y[{made}|{text}]")
    }
}

struct Partial;

impl fmt::Display for Partial {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        print!("open ");
        let mut out = std::io::stdout();
        writeln!(out, "written").unwrap();
        write!(f, "partial")
    }
}

struct Halfway;

impl fmt::Display for Halfway {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "half")?;
        println!("said");
        panic!("in the middle");
    }
}

fn shown<T: fmt::Display>(value: &T) {
    println!("shown <{}>", value);
}

fn main() {
    println!("a {} b {} c", Noisy(1), Noisy(2));
    println!("grid {:?} end", Grid(2));
    println!("outer {} end", Outer);
    println!("partial {} end", Partial);
    print!("no newline {} ", Noisy(3));
    println!("then");
    shown(&Noisy(4));
    let caught = std::panic::catch_unwind(|| println!("before {} after", Halfway));
    println!("caught {}", caught.is_err());
    println!("plain {}", 6);
}
