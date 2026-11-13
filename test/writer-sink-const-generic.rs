// A writer generic over a constant, as arrayvec's `ArrayString<CAP>` is,
// written a piece at a time, directly and through its `TryFrom<Arguments>`,
// whose `write_fmt` of what it's given writes each piece; one that fills
// up fails part way, what came before it kept (#1652, ADR 1135).
use std::fmt::{self, Write};

pub struct Small<const CAP: usize> {
    text: String,
}

impl<const CAP: usize> Small<CAP> {
    pub fn new() -> Self {
        Small { text: String::new() }
    }
}

impl<const CAP: usize> Write for Small<CAP> {
    fn write_str(&mut self, s: &str) -> fmt::Result {
        if self.text.len() + s.len() > CAP {
            return Err(fmt::Error);
        }
        self.text.push_str(s);
        Ok(())
    }

    fn write_char(&mut self, c: char) -> fmt::Result {
        self.write_str(c.encode_utf8(&mut [0; 4]))
    }
}

impl<'a, const CAP: usize> TryFrom<fmt::Arguments<'a>> for Small<CAP> {
    type Error = fmt::Error;

    fn try_from(f: fmt::Arguments<'a>) -> Result<Self, Self::Error> {
        let mut v = Self::new();
        v.write_fmt(f)?;
        Ok(v)
    }
}

// A constant, and an impl's, whose body no writer is (an ICE once,
// `fn_sig` of one).
const LIMIT: usize = 8;

trait Capacity {
    const SIZE: usize;
}

impl<const CAP: usize> Capacity for Small<CAP> {
    const SIZE: usize = CAP;
}

fn main() {
    let n = -4;
    let s: Small<LIMIT> = Small::try_from(format_args!("[{n:>3}]")).unwrap();
    let mut t: Small<3> = Small::new();
    let r = write!(t, "{}{}", "ab", n);
    println!("{:?} {:?} {:?} {}", s.text, t.text, r, Small::<3>::SIZE);
}
