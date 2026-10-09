// A `String`'s edits of a byte range, and `str`'s searches and splits from
// the end, by UTF-8 bytes as Rust's are (ADR 0323).

fn main() {
    let mut text = String::from("héllo wörld");
    let tail = text.split_off(6);
    println!("[{text}] [{tail}]");
    text.replace_range(1..3, "e");
    println!("[{text}]");
    let mut long = String::from("aé-bc-déf");
    let taken: String = long.drain(..4).collect();
    println!("[{taken}] [{long}]");
    let removed: Vec<char> = long.drain(2..=3).collect();
    println!("{removed:?} [{long}]");
    long.extend_from_within(..2);
    long.extend_from_within(1..);
    println!("[{long}]");

    println!("{} {} {}", "abc".is_ascii(), "héllo".is_ascii(), "".is_ascii());
    println!("[{}] [{}] [{}]", " \t a b \n".trim_ascii(), "  x ".trim_ascii_start(), "  x ".trim_ascii_end());
    println!("{:?}", "\x0bx\x0b ".trim_ascii());
    println!("{:?}", "a\nb\n\nc\n".split_inclusive('\n').collect::<Vec<_>>());
    println!("{:?}", "a--b--".split_inclusive("--").collect::<Vec<_>>());
    println!("{:?} {:?}", "ab".split_inclusive("").collect::<Vec<_>>(), "".split_inclusive('x').collect::<Vec<_>>());
    println!("{:?}", "a.b.c.".rsplit_terminator('.').collect::<Vec<_>>());
    println!("{:?} {:?}", "abaXab".rmatches("ab").collect::<Vec<_>>(), "aaa".rmatch_indices("aa").collect::<Vec<_>>());
    println!("{:?}", "héllo".rmatch_indices('l').collect::<Vec<_>>());
    println!("{:?} {:?} {:?}", "héllo".split_at_checked(1), "héllo".split_at_checked(2), "héllo".split_at_checked(9));
    println!("{:?}", "hé𝄞".encode_utf16().collect::<Vec<u16>>());
    println!("{:?} {:?}", String::from_utf16(&[104, 0xD834, 0xDD1E]), String::from_utf16(&[0xD834, 104]));
    println!("{}", String::from_utf16(&[0xDD1E]).unwrap_err());
    println!("{}", String::from_utf16_lossy(&[104, 0xD834, 105]));
    println!("{} {}", "héllo".floor_char_boundary(2), "héllo".ceil_char_boundary(2));
    println!("{} {}", "héllo".floor_char_boundary(99), "héllo".ceil_char_boundary(99));
}
