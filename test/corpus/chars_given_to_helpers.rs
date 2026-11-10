// A string's `chars()` given to code that steps it as it will (#1186):
// its back, by a helper or a struct's method that takes it, by std's
// adapters that hold it, or through a `dyn`; its hint, by generic code.
// Each is the runtime's `$chars(s)`, which knows its back and its hint,
// as JS's own string iterator doesn't.
use std::iter::Peekable;
use std::str::Chars;

fn lex(mut c: Chars<'_>) -> Option<char> {
    c.next_back()
}

fn trim_end(c: &mut Chars<'_>) {
    c.next_back();
}

fn hint<I: Iterator>(it: I) -> (usize, Option<usize>) {
    it.size_hint()
}

fn back<I: DoubleEndedIterator<Item = char>>(mut it: I) -> Option<char> {
    it.next_back()
}

fn second_last<I: DoubleEndedIterator<Item = (usize, char)>>(mut it: I) -> Option<(usize, char)> {
    it.nth_back(1)
}

struct Lexer<'a> {
    chars: Peekable<Chars<'a>>,
}

impl Lexer<'_> {
    fn digits(&mut self) -> String {
        let mut out = String::new();
        while let Some(c) = self.chars.next_if(|c| c.is_ascii_digit()) {
            out.push(c);
        }
        out
    }
}

struct Ends<'a> {
    chars: Chars<'a>,
}

impl Ends<'_> {
    fn last(&mut self) -> Option<char> {
        self.chars.next_back()
    }
}

fn main() {
    for s in ["", "héllo", "12ab34", "日本語😀"] {
        println!("{:?}", lex(s.chars()));
        let mut c = s.chars();
        trim_end(&mut c);
        println!("{:?}", c.collect::<String>());

        let mut lexer = Lexer { chars: s.chars().peekable() };
        println!("{:?}", lexer.digits());
        let mut ends = Ends { chars: s.chars() };
        println!("{:?} {:?}", ends.last(), ends.chars.next());

        println!("{:?} {:?}", hint(s.chars()), hint(s.chars().map(|c| c.len_utf8())));
        let mut held = s.chars();
        held.next();
        println!("{:?}", hint(held));

        println!("{:?} {:?}", back(s.chars()), back(s.chars().filter(|c| *c != 'l')));
        println!("{:?}", second_last(s.char_indices()));

        let mut fused = s.chars().fuse();
        println!("{:?} {:?}", fused.next_back(), fused.next());
        let mut boxed: Box<dyn DoubleEndedIterator<Item = char>> = Box::new(s.chars());
        println!("{:?} {:?}", boxed.next_back(), boxed.next());
    }
}
