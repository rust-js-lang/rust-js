// The crate's own `fmt` traits besides `Display` and `Debug`: `{:x}`, `{:X}`,
// `{:o}`, `{:b}`, `{:e}` and `{:p}` of its types call its impls, given the
// placeholder's options, as uuid's and thiserror's are. And `write_str`
// called through `fmt::Write` on a `Formatter`, as bitflags writes.

use std::fmt::{self, Binary, LowerExp, LowerHex, Octal, Pointer, UpperHex, Write};

struct Id(u32);

impl LowerHex for Id {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        if f.alternate() {
            f.write_str("0x")?;
        }
        write!(f, "{:08x}", self.0)
    }
}

impl UpperHex for Id {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(f, "{:08X}", self.0)
    }
}

// Delegating to another of the crate's impls, as uuid's do.
impl Octal for Id {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        LowerHex::fmt(self, f)?;
        f.write_str("/")?;
        UpperHex::fmt(self, f)
    }
}

impl Binary for Id {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        f.pad(&format!("{:b}", self.0))
    }
}

struct Ratio(f64);

impl LowerExp for Ratio {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(f, "{:e}~", self.0)
    }
}

struct Handle(usize);

impl Pointer for Handle {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(f, "@{}", self.0)
    }
}

struct Flags(u8);

impl fmt::Display for Flags {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        let mut first = true;
        for (bit, name) in [(1, "A"), (2, "B"), (4, "C")] {
            if self.0 & bit != 0 {
                if !first {
                    f.write_str(" | ")?;
                }
                Write::write_str(f, name)?;
                first = false;
            }
        }
        f.write_char('.')
    }
}

fn main() {
    let id = Id(48879);
    println!("{:x} {:#x} {:X} {:o} [{:>10b}]", id, id, id, id, Id(5));
    println!("{:e} {:p}", Ratio(1234.5), Handle(7));
    println!("{} {}", Flags(5), Flags(7));
    let shown = format!("{:x}-{}", Id(255), Flags(2));
    println!("{shown}");
}
