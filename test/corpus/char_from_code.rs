// `char::from_u32_unchecked`, as utf8_iter's decoder calls it on a code point
// it checked itself: the `char` it is, as `from_u32` gives it.

fn decode(bytes: &[u8]) -> Vec<char> {
    let mut out = Vec::new();
    let mut i = 0;
    while i < bytes.len() {
        let b = bytes[i] as u32;
        let (point, width) = if b < 0x80 {
            (b, 1)
        } else if b < 0xE0 {
            (((b & 0x1F) << 6) | (bytes[i + 1] as u32 & 0x3F), 2)
        } else if b < 0xF0 {
            (((b & 0x0F) << 12) | ((bytes[i + 1] as u32 & 0x3F) << 6) | (bytes[i + 2] as u32 & 0x3F), 3)
        } else {
            (
                ((b & 0x07) << 18)
                    | ((bytes[i + 1] as u32 & 0x3F) << 12)
                    | ((bytes[i + 2] as u32 & 0x3F) << 6)
                    | (bytes[i + 3] as u32 & 0x3F),
                4,
            )
        };
        // SAFETY: a code point of well-formed UTF-8.
        out.push(unsafe { char::from_u32_unchecked(point) });
        i += width;
    }
    out
}

fn main() {
    let chars = decode("aé€😀".as_bytes());
    println!("{:?} {}", chars, chars.len());
    println!("{:?}", chars.iter().map(|&c| c == char::from_u32(c as u32).unwrap()).collect::<Vec<_>>());
}
