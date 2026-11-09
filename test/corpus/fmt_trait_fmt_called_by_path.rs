//@ library-differs
// A formatting trait's `fmt` called by its path, `fmt::LowerHex::fmt(self,
// f)`, goes where `{:x}` of the value goes, the `Formatter` it's given, its
// flags, width and precision, passed on (#1115): std's own impls, a user
// impl, a generic one as libm's `Hexf<T>`'s `Debug` forwards to its
// `LowerHex`, `f.alternate()` read through it, and an error the callee
// gives, propagated.
use std::fmt::{self, Write};

struct Hexy(u32);

impl fmt::LowerHex for Hexy {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        if f.alternate() {
            f.write_str("hex:")?;
        }
        fmt::LowerHex::fmt(&self.0, f)
    }
}

impl fmt::Debug for Hexy {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        fmt::LowerHex::fmt(self, f)
    }
}

struct Shown(u32);

impl fmt::Display for Shown {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        fmt::Display::fmt(&self.0, f)
    }
}

impl fmt::Debug for Shown {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        fmt::Display::fmt(self, f)
    }
}

// libm's shape: a generic wrapper whose `Debug` forwards to its `LowerHex`.
struct Hexf<T>(T);

impl fmt::LowerHex for Hexf<u8> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "0x{:02x}", self.0)
    }
}

impl fmt::LowerHex for Hexf<(u8, u8)> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "({:x}, {:x})", Hexf(self.0.0), Hexf(self.0.1))
    }
}

impl<T> fmt::Debug for Hexf<T>
where
    Hexf<T>: fmt::LowerHex,
{
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        fmt::LowerHex::fmt(self, f)
    }
}

// A writer that fails past its room: the callee's error is the caller's.
struct Full(usize);

impl Write for Full {
    fn write_str(&mut self, s: &str) -> fmt::Result {
        if s.len() > self.0 {
            return Err(fmt::Error);
        }
        self.0 -= s.len();
        Ok(())
    }
}

fn main() {
    println!("{:?} [{:#06x}] [{:#x}]", Hexy(255), Hexy(255), Hexy(16));
    println!("[{:>5}] [{:<4?}] [{:+}]", Shown(7), Shown(8), Shown(9));
    println!("{:?} {:?}", Hexf(10u8), Hexf((1u8, 255u8)));
    struct Std;
    impl fmt::Display for Std {
        fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
            fmt::Display::fmt(&5u32, f)?;
            f.write_str(" ")?;
            fmt::LowerHex::fmt(&255u8, f)?;
            f.write_str(" ")?;
            fmt::UpperHex::fmt(&255u8, f)?;
            f.write_str(" ")?;
            fmt::Octal::fmt(&8u8, f)?;
            f.write_str(" ")?;
            fmt::Binary::fmt(&5u8, f)?;
            f.write_str(" ")?;
            fmt::LowerExp::fmt(&1500.0f64, f)
        }
    }
    println!("[{}] [{:#06}]", Std, Std);
    let mut full = Full(4);
    println!("{:?} {}", write!(full, "{:?}", Hexy(255)), full.0);
    println!("{:?} {}", write!(full, "{:?}", Hexy(0xabcdef)), full.0);
}
