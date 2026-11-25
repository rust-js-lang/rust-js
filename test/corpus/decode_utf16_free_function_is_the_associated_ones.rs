// `std::char::decode_utf16(units)`, the free function, decodes as
// `char::decode_utf16(units)` does: each `u16` or surrogate pair its
// `char`, an unpaired surrogate its `DecodeUtf16Error` (ADR 0327, #2599).
fn main() {
    let units = [0x61u16, 0xD800, 0x62, 0xD83D, 0xDE00];
    let free: Vec<_> = std::char::decode_utf16(units).collect();
    let associated: Vec<_> = char::decode_utf16(units).collect();
    println!("{free:?}");
    println!("{}", free == associated);
    for r in std::char::decode_utf16(units.iter().copied()) {
        match r {
            Ok(c) => println!("{c}"),
            Err(e) => println!("unpaired {:x}", e.unpaired_surrogate()),
        }
    }
}
