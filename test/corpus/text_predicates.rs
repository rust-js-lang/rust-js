// Searching text by what a character is: a closure, a function such as
// `char::is_numeric`, or a set of characters, `[',', ';']`. And a string's
// pieces got by bytes, its ASCII words, and characters' sizes.

fn main() {
    let line = "id=42; name=Zoë, age 31";

    // A closure, a function, a set: where the first and last match, in bytes.
    println!(
        "{:?} {:?} {:?} {:?}",
        line.find(|c: char| c.is_ascii_digit()),
        line.rfind(char::is_numeric),
        line.find([';', ',']),
        line.find(char::is_whitespace)
    );
    println!("{:?} {:?}", line.find(|c: char| c == 'ë'), line.rfind(|c: char| c == '~'));

    // Starting and ending with one.
    println!(
        "{} {} {} {}",
        line.starts_with(char::is_alphabetic),
        line.ends_with(|c: char| c.is_ascii_digit()),
        "".starts_with(|_: char| true),
        "😀x".starts_with(|c: char| c.len_utf8() == 4)
    );
    println!("{}", "x😀".ends_with(|c: char| c.len_utf8() == 4));

    // Split, tested and trimmed by one.
    let fields: Vec<&str> = line.split([';', ',']).map(str::trim).collect();
    println!("{:?}", fields);
    let words: Vec<&str> = "a1b22c".split(char::is_numeric).collect();
    println!("{:?} {}", words, line.contains(char::is_uppercase));
    println!("{:?}", "--==x==--".trim_matches(['-', '=']));
    println!("{:?}", "  tabs\tand  spaces\n".split_ascii_whitespace().collect::<Vec<_>>());
    // A vertical tab isn't ASCII whitespace to Rust, though JS's `\s` is.
    println!("{:?}", "a\u{b}b c".split_ascii_whitespace().collect::<Vec<_>>());

    // Pieces by bytes, `None` where a range isn't on a character's edge.
    let name = "Zoë!";
    println!(
        "{:?} {:?} {:?} {:?} {:?}",
        name.get(0..2),
        name.get(2..3),
        name.get(2..4),
        name.get(4..),
        name.get(..=1)
    );
    println!("{:?} {:?} {:?}", name.get(..), name.get(3..9), name.get(9..));
    let edges: Vec<bool> = (0..=6).map(|i| name.is_char_boundary(i)).collect();
    println!("{:?}", edges);

    // Characters' sizes, and characters from bytes and back.
    println!("{} {} {} {}", 'a'.len_utf8(), 'ë'.len_utf8(), '😀'.len_utf8(), '😀'.len_utf16());
    println!("{} {}", char::from(65u8), char::from(233u8));
    let letters = String::from_iter(['h', 'é', '!']);
    let joined = String::from_iter(vec!["ab", "cd"]);
    println!("{} {}", letters, joined);
}
