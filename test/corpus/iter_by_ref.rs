// `by_ref()`, and `&mut it`: a `for` loop takes items from an iterator,
// and stops, and what's left is still there, as num-traits' float parser reads a number's digits
// and then its exponent, and bitflags' names of a value step through an iterator of its own.

fn split_number(text: &str) -> (String, String) {
    let mut cs = text.chars().enumerate();
    let mut digits = String::new();
    for (_, c) in cs.by_ref() {
        if c == 'e' {
            break;
        }
        digits.push(c);
    }
    let rest: String = cs.map(|(i, c)| format!("{i}{c}")).collect();
    (digits, rest)
}

fn split_mut(text: &str) -> (String, usize) {
    let mut cs = text.chars();
    let mut head = String::new();
    for c in &mut cs {
        if c == ':' {
            break;
        }
        head.push(c);
    }
    (head, cs.count())
}

struct Names {
    at: usize,
}

impl Iterator for Names {
    type Item = (&'static str, u32);

    fn next(&mut self) -> Option<Self::Item> {
        let all = [("A", 1), ("B", 2), ("AB", 3), ("C", 4)];
        let item = all.get(self.at).copied();
        self.at += 1;
        item
    }
}

struct EqualNames {
    inner: Names,
    bits: u32,
}

impl Iterator for EqualNames {
    type Item = &'static str;

    fn next(&mut self) -> Option<Self::Item> {
        for (n, bits) in self.inner.by_ref() {
            if bits == self.bits {
                return Some(n);
            }
        }
        None
    }
}

fn main() {
    println!("{:?} {:?}", split_number("12e34"), split_number("7"));
    println!("{:?} {:?}", split_mut("key:value"), split_mut("none"));
    let names: Vec<&str> = EqualNames { inner: Names { at: 0 }, bits: 3 }.collect();
    println!("{names:?}");
    let mut all = Names { at: 0 };
    for (n, _) in all.by_ref() {
        if n == "B" {
            break;
        }
    }
    println!("{:?} {}", all.next(), all.at);
}
