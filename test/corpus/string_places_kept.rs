// An offset the program can see as a number keeps counting UTF-8 bytes, by
// ADR 0138's helpers: printed, returned, stored, compared with a number,
// used on another string, or stepped by what isn't what was found (ADR
// 0642). And one only a place, beside them, JS's units.

struct Span {
    start: usize,
}

fn found(s: &str) -> Option<usize> {
    s.find('é')
}

fn on_another<'a>(s: &str, other: &'a str) -> &'a str {
    match s.find(' ') {
        Some(at) => &other[..at],
        None => other,
    }
}

fn stepped(s: &str) -> &str {
    match s.find('a') {
        Some(at) if at + 3 <= s.len() => &s[at + 3..],
        _ => "",
    }
}

fn near_start(s: &str) -> bool {
    match s.find('b') {
        Some(at) => at < 3,
        None => false,
    }
}

fn between(s: &str) -> &str {
    let (Some(open), Some(close)) = (s.find('('), s.find(')')) else {
        return "";
    };
    if open < close { &s[open + 1..close] } else { "" }
}

fn after_colon(s: &str) -> Option<&str> {
    s.find(':').map(|at| &s[at + 1..])
}

fn unwrapped(s: &str) -> &str {
    let at = s.find('=').unwrap();
    &s[..at]
}

fn edited(mut s: String) -> String {
    if let Some(at) = s.find('é') {
        s.insert(at, '[');
    }
    s
}

fn main() {
    for s in ["hello", "héllo", "🦀é", "nope"] {
        println!("{:?} {:?}", found(s), s.find('l'));
        let span = Span { start: s.find('l').unwrap_or(0) };
        println!("{}", span.start);
    }
    println!("{} {}", on_another("ab cd", "wxyz"), on_another("🦀 x", "abcdefgh"));
    println!("[{}] [{}] [{}]", stepped("xaé🦀yz"), stepped("abcdef"), stepped("xa"));
    println!("{} {} {}", near_start("ab"), near_start("ééb"), near_start("é"));
    println!("[{}] [{}] [{}]", between("f(é🦀)"), between(")("), between("(x"));
    println!("{:?} {:?} {:?}", after_colon("a:é"), after_colon("🦀:🦀"), after_colon("none"));
    println!("{} {}", unwrapped("é=1"), unwrapped("🦀🦀=x"));
    println!("{}", edited(String::from("héllo")));
    let words: Vec<(usize, char)> = "aé🦀".char_indices().collect();
    println!("{:?}", words);
}
