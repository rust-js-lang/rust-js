//@ library-differs
// A writer's own `write_fmt`, which reads its `fmt::Arguments` as text or
// writes it a piece at a time, called where the call can't name it: through
// a type parameter bounded by `fmt::Write`, through a `&mut dyn fmt::Write`,
// and by `write!`. Each is the crate's own function, given the `Arguments`'
// function, whatever path the call takes (#1695, #2058).
use std::fmt::{self, Write};

struct Lengths(Vec<usize>);

impl Write for Lengths {
    fn write_str(&mut self, s: &str) -> fmt::Result {
        self.0.push(s.len());
        Ok(())
    }

    fn write_fmt(&mut self, args: fmt::Arguments<'_>) -> fmt::Result {
        self.0.push(1000 + args.to_string().len());
        Ok(())
    }
}

struct Pieces(String);

impl Write for Pieces {
    fn write_str(&mut self, s: &str) -> fmt::Result {
        self.0.push_str(s);
        self.0.push('/');
        Ok(())
    }

    fn write_fmt(&mut self, args: fmt::Arguments<'_>) -> fmt::Result {
        self.0.push('{');
        fmt::write(self, args)?;
        self.0.push('}');
        Ok(())
    }
}

fn generic<W: Write>(w: &mut W, args: fmt::Arguments<'_>) -> fmt::Result {
    w.write_fmt(args)
}

fn dynamic(w: &mut dyn Write, args: fmt::Arguments<'_>) -> fmt::Result {
    w.write_fmt(args)
}

fn main() {
    let n = 42;
    let mut lengths = Lengths(Vec::new());
    generic(&mut lengths, format_args!("a{n}b")).unwrap();
    dynamic(&mut lengths, format_args!("{n}{n}")).unwrap();
    write!(lengths, "x{}y{}", n, "z").unwrap();
    let held = format_args!("[{n:>5}]");
    generic(&mut lengths, held).unwrap();
    println!("{:?}", lengths.0);

    let mut pieces = Pieces(String::new());
    generic(&mut pieces, format_args!("a{n}b")).unwrap();
    dynamic(&mut pieces, format_args!("{n}-{n}")).unwrap();
    write!(pieces, "x{}y", n).unwrap();
    let held = format_args!("<{}>", n + 1);
    dynamic(&mut pieces, held).unwrap();
    println!("{}", pieces.0);
}
