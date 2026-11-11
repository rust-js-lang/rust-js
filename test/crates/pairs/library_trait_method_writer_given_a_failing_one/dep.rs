//! A library's trait whose method is generic over a writer, and its impl,
//! which a dictionary calls, that no consumer compiles for its own: a write
//! through the method's own type parameter may fail (#1392).
use std::io::{self, Write};

pub trait Render {
    fn render<W: Write>(&self, out: &mut W) -> io::Result<()>;

    fn rendered<W: Write>(&self, out: &mut W) -> bool {
        self.render(out).is_ok()
    }
}

pub struct Dot;

impl Render for Dot {
    fn render<W: Write>(&self, out: &mut W) -> io::Result<()> {
        out.write_all(b".")
    }
}

pub struct Dash;

/// Its own code asks for each impl's dictionary, so its consumer calls the
/// library's.
pub fn drawn(out: &mut Vec<u8>) -> usize {
    count(&[Dot], out) + count(&[Dash], out)
}

fn count<R: Render, W: Write>(items: &[R], out: &mut W) -> usize {
    items.iter().filter(|item| item.rendered(out)).count()
}

impl Render for Dash {
    fn render<W: Write>(&self, out: &mut W) -> io::Result<()> {
        let written = out.write_all(b"-");
        if written.is_err() {
            println!("dash refused");
        }
        written
    }
}
