// The `&mut str` `encode_utf8` gives is the buffer's bytes: a change
// through it is written there, and it reads as the `char`'s text alone.
fn main() {
    let mut buf = [0u8; 4];
    let s = 'q'.encode_utf8(&mut buf);
    s.make_ascii_uppercase();
    println!("{s}");
    println!("{buf:?}");

    let mut out = String::new();
    let mut b = [0u8; 4];
    for c in ['h', 'é', '😀'] {
        out.push_str(c.encode_utf8(&mut b));
    }
    println!("{out} {}", out.len());
}
