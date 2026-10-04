// `{:p}`, `{:x}` and the like through generic code, as thiserror's `Var`
// forwards them: a `T: Pointer`'s or a `T: LowerHex`'s dictionary, given
// the placeholder's options.

use std::fmt::{self, LowerHex, Pointer};

pub struct Var<'a, T: ?Sized>(pub &'a T);

impl<'a, T: Pointer + ?Sized> Pointer for Var<'a, T> {
    fn fmt(&self, formatter: &mut fmt::Formatter) -> fmt::Result {
        Pointer::fmt(self.0, formatter)
    }
}

impl<'a, T: LowerHex + ?Sized> LowerHex for Var<'a, T> {
    fn fmt(&self, formatter: &mut fmt::Formatter) -> fmt::Result {
        LowerHex::fmt(self.0, formatter)
    }
}

struct Handle(u32);

impl Pointer for Handle {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(f, "@{}", self.0)
    }
}

impl LowerHex for Handle {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        if f.alternate() {
            f.write_str("0x")?;
        }
        write!(f, "{:x}", self.0)
    }
}

fn hex<T: LowerHex>(t: &T) -> String {
    format!("{:x}|{:#x}", t, t)
}

fn main() {
    println!("{:p} {:x} {:#x}", Var(&Handle(7)), Var(&Handle(255)), Var(&Handle(16)));
    println!("{} {}", hex(&Handle(10)), hex(&Var(&Handle(11))));
}
