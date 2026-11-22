// What `print!` leaves unended, std's stdout `LineWriter` holds till the
// line's newline, a `flush()` or the program's end, while stderr is
// written at once: `print!("a")` then `eprintln!("err 1")` shows `err 1`
// first (#2319). Bytes written by `write_all` are held so too.
use std::io::Write;

fn main() {
    print!("a");
    eprintln!("err 1");
    print!("b\nc");
    eprintln!("err 2");
    std::io::stdout().flush().unwrap();
    eprintln!("err 3");
    print!("d");
    std::io::stdout().write_all(b"e\nf").unwrap();
    eprint!("err 4\n");
    println!("g");
    let mut out = std::io::stdout();
    write!(out, "h").unwrap();
    out.flush().unwrap();
    eprintln!("err 5");
    print!("left at the end");
}
