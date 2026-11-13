// An `impl Display` a writer gives its `Formatter`, anstyle's
// `self.effects.render().fmt(f)`, is the type it hides, whose writer is
// framed and writes a piece at a time (#1652, ADR 1135). Apart, as a writer
// any type may be, a `dyn`'s or a held `Formatter`'s, frames every writer.
use std::fmt::{self, Display, Write};

// Records each piece it's given.
struct Log(Vec<String>);

impl Write for Log {
    fn write_str(&mut self, s: &str) -> fmt::Result {
        self.0.push(s.to_string());
        Ok(())
    }
}

// Fails at its third call.
struct Failing(Vec<String>);

impl Write for Failing {
    fn write_str(&mut self, s: &str) -> fmt::Result {
        if self.0.len() == 2 {
            return Err(fmt::Error);
        }
        self.0.push(s.to_string());
        Ok(())
    }
}

struct Effects(u8);

struct EffectsDisplay(u8);

impl Display for EffectsDisplay {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        for i in 0..self.0 {
            f.write_str(["b", "i", "u"][usize::from(i % 3)])?;
        }
        Ok(())
    }
}

impl Effects {
    fn render(&self) -> impl Display {
        EffectsDisplay(self.0)
    }
}

struct Style(Effects);

impl Display for Style {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str("[")?;
        self.0.render().fmt(f)?;
        f.write_str("]")
    }
}

fn main() {
    let mut log = Log(Vec::new());
    write!(log, "{}|{}", Style(Effects(4)), Style(Effects(0))).unwrap();
    println!("{:?} {}", log.0, Style(Effects(2)));
    let mut failing = Failing(Vec::new());
    let r = write!(failing, "{}", Style(Effects(3)));
    println!("{:?} {:?}", failing.0, r);
}
