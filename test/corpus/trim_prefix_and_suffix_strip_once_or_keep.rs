// A `str`'s or a slice's `trim_prefix(p)` and `trim_suffix(p)`, Rust
// 1.100's: its strip by `p`, else itself, by text, a `char`, a `char`'s
// predicate, a set of `char`s and a slice's items, of a `String` and of
// text not all ASCII, as native's (#1656).
fn strip_name(s: &str) -> &str {
    s.trim_prefix("name: ")
}
fn tail_of(s: String) -> String {
    s.as_str().trim_suffix(".rs").to_string()
}
fn main() {
    println!(
        "{:?} {:?}",
        "prefix-body".trim_prefix("prefix-"),
        "body".trim_prefix("prefix-")
    );
    println!("{:?} {:?}", "body.rs".trim_suffix(".rs"), "body.js".trim_suffix(".rs"));
    println!("{:?} {:?}", "xxab".trim_prefix('x'), "abyy".trim_suffix('y'));
    println!(
        "{:?} {:?}",
        "9abc".trim_prefix(|c: char| c.is_ascii_digit()),
        "abc!".trim_suffix(['!', '?'])
    );
    println!("{:?} {:?}", strip_name("name: lisa"), tail_of(String::from("main.rs")));
    println!("{:?}", "".trim_prefix(""));
    let v = [1u8, 2, 3, 4];
    println!("{:?} {:?}", v.trim_prefix(&[1, 2]), v.trim_suffix(&[9]));
    println!("{:?} {:?}", v.trim_prefix(&[1u8, 2][..]), v.trim_suffix(&[3, 4]));
    let words = vec![String::from("a"), String::from("b")];
    println!("{:?}", words.trim_prefix(&[String::from("a")]));
    let s = String::from("héllo wörld");
    println!("{:?} {:?}", s.trim_prefix("hé"), s.trim_suffix("ld"));
}
