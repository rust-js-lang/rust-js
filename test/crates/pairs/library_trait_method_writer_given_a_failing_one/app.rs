//! A writer of its own given to a library impl's generic method, by name
//! and through generic code of its own.
use dep::{Dash, Dot, Render};
use std::io::{self, Write};

struct Refusing(u8);

impl Write for Refusing {
    fn write(&mut self, bytes: &[u8]) -> io::Result<usize> {
        match self.0 {
            0 => Err(io::Error::other("refused")),
            _ => Ok(bytes.len()),
        }
    }

    fn flush(&mut self) -> io::Result<()> {
        Ok(())
    }
}

fn count<R: Render>(items: &[R], out: &mut Refusing) -> usize {
    items.iter().filter(|item| item.rendered(out)).count()
}

pub fn main() {
    let mut refusing = Refusing(0);
    let mut taking = Refusing(1);
    println!("{:?}", Dot.render(&mut refusing).map_err(|e| e.to_string()));
    println!("{} {}", Dot.rendered(&mut refusing), Dot.rendered(&mut taking));
    println!("{:?}", Dash.render(&mut refusing).is_err());
    println!("{} {}", count(&[Dash, Dash], &mut refusing), count(&[Dot], &mut taking));
    let mut bytes = Vec::new();
    println!("{} {bytes:?}", dep::drawn(&mut bytes));
}
