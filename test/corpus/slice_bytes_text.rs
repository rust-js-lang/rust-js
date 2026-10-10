// A byte slice's `escape_ascii()`, its text as `\n`, `\x7f` and the rest,
// and `utf8_chunks()`, each run of valid text and the bad bytes after it.

fn main() {
    let bytes = b"tab\there \"quoted\" 'it' \\ \x7f\x00\xff~";
    println!("{}", bytes.escape_ascii());
    let escaped = b"a\nb".escape_ascii().to_string();
    println!("{escaped} {}", escaped.len());
    println!("[{}]", b"".escape_ascii());

    let mixed = b"ok\xffh\xc3\xa9\xe2\x82 end\xf0\x9f";
    for chunk in mixed.utf8_chunks() {
        println!("{:?} {:?}", chunk.valid(), chunk.invalid());
    }
    let text: String = b"caf\xc3\xa9\xc3".utf8_chunks().map(|c| c.valid()).collect();
    println!("{text} {}", b"".utf8_chunks().count());
}
