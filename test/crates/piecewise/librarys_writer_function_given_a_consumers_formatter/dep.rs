//! A library's writer function, which no writer of its own calls: framed,
//! as a consumer's writer may give it its `Formatter`, arming its sink for
//! it (#1652, ADR 1135).
use std::fmt;

pub struct Listed(pub Vec<u32>);

impl Listed {
    pub fn joined(&self, f: &mut fmt::Formatter<'_>, sep: &str) -> fmt::Result {
        for (i, x) in self.0.iter().enumerate() {
            if i > 0 {
                f.write_str(sep)?;
            }
            write!(f, "{x:>3}")?;
        }
        Ok(())
    }
}
