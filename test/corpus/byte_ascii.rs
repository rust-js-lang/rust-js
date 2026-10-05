// A byte's ASCII tests and case changes, as chrono's parser reads its
// input: `b.is_ascii_digit()`.

fn digits(text: &str) -> u32 {
    let mut n = 0;
    for &b in text.as_bytes() {
        if !b.is_ascii_digit() {
            break;
        }
        n = n * 10 + u32::from(b - b'0');
    }
    n
}

fn main() {
    println!("{}", digits("2024-10-05"));
    let tests: [(&str, fn(&u8) -> bool); 11] = [
        ("digit", u8::is_ascii_digit),
        ("hexdigit", u8::is_ascii_hexdigit),
        ("alphabetic", u8::is_ascii_alphabetic),
        ("alphanumeric", u8::is_ascii_alphanumeric),
        ("uppercase", u8::is_ascii_uppercase),
        ("lowercase", u8::is_ascii_lowercase),
        ("whitespace", u8::is_ascii_whitespace),
        ("punctuation", u8::is_ascii_punctuation),
        ("graphic", u8::is_ascii_graphic),
        ("control", u8::is_ascii_control),
        ("ascii", u8::is_ascii),
    ];
    for (name, test) in tests {
        let count = (0..=255u8).filter(|b| test(b)).count();
        let first = (0..=255u8).find(|b| test(b)).unwrap_or(0);
        let last = (0..=255u8).rev().find(|b| test(b)).unwrap_or(0);
        println!("{name} {count} {first} {last}");
    }
    let b = b'g';
    println!("{} {} {}", b.is_ascii_alphabetic(), b.is_ascii_hexdigit(), b'F'.is_ascii_hexdigit());
    println!("{} {}", b' '.is_ascii_whitespace(), 11u8.is_ascii_whitespace());
    println!("{} {} {}", b.to_ascii_uppercase(), b'Q'.to_ascii_lowercase(), b'7'.to_ascii_uppercase());
    let word: Vec<u8> = b"Hello, World!".iter().map(|b| b.to_ascii_uppercase()).collect();
    println!("{}", String::from_utf8(word).unwrap());
}
