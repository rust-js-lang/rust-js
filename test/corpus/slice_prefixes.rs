// `starts_with` and `ends_with` of a slice whose items `==` compares by
// value, and `eq_ignore_ascii_case` of bytes, as uuid's and writeable's are.

fn main() {
    let v = [1, 2, 3, 4];
    println!("{} {} {} {}", v.starts_with(&[1, 2]), v.starts_with(&[2]), v.starts_with(&[]), v.starts_with(&[1, 2, 3, 4, 5]));
    println!("{} {} {}", v.ends_with(&[3, 4]), v.ends_with(&[1, 2, 3, 4, 5]), v[..2].ends_with(&[2]));
    let words = vec!["a", "b", "c"];
    println!("{} {}", words.starts_with(&["a", "b"]), words.ends_with(&["b"]));
    let text = "content-type: json";
    println!("{} {}", text.as_bytes().starts_with(b"content"), text.as_bytes().ends_with(b"xml"));
    let flags = [true, false];
    println!("{}", flags.ends_with(&[false]));

    let a: &[u8] = b"Content-Type";
    println!("{} {}", a.eq_ignore_ascii_case(b"content-type"), a.eq_ignore_ascii_case(b"content_type"));
    println!("{} {}", b"ab".eq_ignore_ascii_case(b"abc"), [0xC0u8, 0x80].eq_ignore_ascii_case(&[0xE0, 0x80]));
}
