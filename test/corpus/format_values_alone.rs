// A format string of values alone, `{}` or `{}{}`, is text: of one value,
// its text, and of two, both texts joined, never what `+` of two numbers
// is. Read by its length, pushed into a `Vec<String>`, compared with a
// `String`, a map's key, its `{:?}`; a `+`'s operands stay as `+` joins
// them, and a write that fails part way keeps what came before (ADR 0066).
use std::collections::HashMap;
use std::fmt::{self, Display, Write};

struct Hours(u32);

impl Display for Hours {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        if self.0 > 23 {
            return Err(fmt::Error);
        }
        write!(f, "{:02}h", self.0)
    }
}

fn joined(a: i32, b: i32) -> String {
    format!("{}{}", a, b)
}

fn alone(n: i32) -> String {
    format!("{}", n)
}

fn main() {
    let (a, b) = (1, 2);
    let both = joined(a, b);
    println!("{both} {} {}", both.len(), alone(a).len());
    let mut out: Vec<String> = Vec::new();
    out.push(format!("{}", a + b));
    out.push(format!("{}{}", a, b));
    out.push(format!("{a}{b}"));
    out.push(format!("{:#x}", 255));
    out.push(format!("{:#b}{}", 5, a));
    out.push(format!("{:#010b}", 5));
    println!("{out:?} {}", out.iter().map(|s| s.len()).sum::<usize>());
    println!("{} {}", joined(a, b) == "12", format!("{}", a) == String::from("1"));
    let mut seen: HashMap<String, usize> = HashMap::new();
    for (x, y) in [(1, 2), (12, 0), (3, 4), (1, 20)] {
        *seen.entry(format!("{}{}", x, y)).or_default() += 1;
    }
    let mut keys: Vec<_> = seen.into_iter().collect();
    keys.sort();
    println!("{keys:?}");
    println!("{:?} {:?}", format!("{}{}", a, b), format!("{}", 7.5));
    let word = String::from("x") + "y";
    println!("{}", format!("{}!", word + "z"));
    let mut s = String::from(">");
    let r = write!(s, "{}{}{}", a, Hours(5), Hours(30));
    println!("{} {s:?}", r.is_err());
}
