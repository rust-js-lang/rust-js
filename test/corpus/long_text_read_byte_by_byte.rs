// A long text read a byte at a time, as a parser reads it (#1184): each
// step's `as_bytes()`, `len()` and `&s[i..]` cost what Rust's do, nothing
// that grows with the text. Each counted the whole text again as JS, so
// semver's 5.6 MB identifier took past twenty minutes to parse: of these
// texts, each loop would take minutes, past the corpus's time for a case.

// semver's `identifier`: a segment's bytes, looked up each step.
fn identifier(input: &str) -> (&str, &str) {
    let mut accumulated_len = 0;
    let mut segment_len = 0;
    loop {
        match input.as_bytes().get(accumulated_len + segment_len) {
            Some(b'A'..=b'Z') | Some(b'a'..=b'z') | Some(b'-') | Some(b'0'..=b'9') => {
                segment_len += 1;
            }
            boundary => {
                accumulated_len += segment_len;
                if boundary == Some(&b'.') {
                    accumulated_len += 1;
                    segment_len = 0;
                } else {
                    return input.split_at(accumulated_len);
                }
            }
        }
    }
}

// A cursor whose methods ask its text each step.
struct Cursor<'a> {
    input: &'a str,
    pos: usize,
}

impl<'a> Cursor<'a> {
    fn peek(&self) -> Option<u8> {
        self.input.as_bytes().get(self.pos).copied()
    }

    fn at_end(&self) -> bool {
        self.pos >= self.input.len()
    }

    fn rest(&self) -> &'a str {
        &self.input[self.pos..]
    }
}

fn spaces(input: &str) -> (usize, usize) {
    let mut cursor = Cursor { input, pos: 0 };
    let mut spaces = 0;
    while !cursor.at_end() {
        if cursor.peek() == Some(b' ') {
            spaces += 1;
        }
        cursor.pos += 1;
        if cursor.pos == input.len() / 2 {
            break;
        }
    }
    (spaces, cursor.rest().len())
}

// Its length asked each step, and a byte by its index.
fn digits(input: &str) -> usize {
    let mut i = 0;
    while i < input.len() && input.as_bytes()[i].is_ascii_digit() {
        i += 1;
    }
    i
}

// A text of characters of each width, each one's bytes read by its first
// byte, as a UTF-8 decoder reads them, and sliced.
fn widths(input: &str) -> ([usize; 4], usize) {
    let mut counts = [0; 4];
    let mut i = 0;
    let mut tail = 0;
    while i < input.len() {
        let width = match input.as_bytes()[i] {
            0..=0x7f => 1,
            0xc0..=0xdf => 2,
            0xe0..=0xef => 3,
            _ => 4,
        };
        counts[width - 1] += 1;
        tail += input[i + width..].len() % 7;
        assert_eq!(input[i..i + width].len(), width);
        i += width;
    }
    (counts, tail)
}

fn main() {
    let n = 100_000;
    let long = format!("{}.{}+build", "1".repeat(n), "rc-".repeat(n));
    let (identifier, rest) = identifier(&long);
    println!("{} {rest}", identifier.len());
    println!("{:?}", spaces(&"ab ".repeat(n)));
    println!("{}", digits(&format!("{}x", "7".repeat(2 * n))));
    println!("{:?}", widths(&"aé日🦀".repeat(n / 4)));
}
