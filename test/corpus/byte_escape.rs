// A byte's escape, `ascii::escape_default(b)` and `b.escape_ascii()`: its
// bytes, iterated, counted and shown, as aho-corasick's `DebugByte` reads
// them.

struct DebugByte(u8);

impl std::fmt::Debug for DebugByte {
    fn fmt(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
        if self.0 == b' ' {
            return write!(f, "' '");
        }
        let mut bytes = [0u8; 10];
        let mut len = 0;
        for (i, mut b) in std::ascii::escape_default(self.0).enumerate() {
            if i >= 2 && b'a' <= b && b <= b'f' {
                b -= 32;
            }
            bytes[len] = b;
            len += 1;
        }
        write!(f, "{}", std::str::from_utf8(&bytes[..len]).unwrap())
    }
}

fn main() {
    for b in [b'a', b' ', b'\n', b'\'', 0x7f, 0xfe, b'\\'] {
        let escaped: Vec<u8> = b.escape_ascii().collect();
        println!(
            "{:?} {} {} {:?}",
            DebugByte(b),
            b.escape_ascii(),
            std::ascii::escape_default(b).len(),
            escaped
        );
    }
    let mut rest = b'\t'.escape_ascii();
    rest.next();
    let back = rest.clone().next_back();
    println!("{rest} {back:?}");
}
