use hex::{FromHex, FromHexError, ToHex};

pub fn report() -> String {
    let mut lines = Vec::new();
    for bytes in [&b""[..], b"\x00\xff", b"Hello, world!", &[0x12, 0xab, 0xcd, 0xef]] {
        let lower = hex::encode(bytes);
        let upper = hex::encode_upper(bytes);
        let via_trait: String = bytes.encode_hex();
        let decoded = hex::decode(&lower);
        lines.push(format!("{lower} {upper} {via_trait} {decoded:?}"));
    }
    for text in ["48656c6c6f", "48656C6C6F", "4865z", "486", "", "0x12"] {
        let decoded = hex::decode(text);
        let fixed = <[u8; 2]>::from_hex(text);
        lines.push(format!("{text:?}: {decoded:?} {fixed:?}"));
    }
    let mut buf = [0u8; 4];
    lines.push(format!("{:?} {buf:?}", hex::decode_to_slice("deadbeef", &mut buf)));
    let mut out = [0u8; 6];
    lines.push(format!("{:?} {out:?}", hex::encode_to_slice(b"abc", &mut out)));
    let errors = [
        FromHexError::InvalidHexCharacter { c: 'g', index: 3 },
        FromHexError::OddLength,
        FromHexError::InvalidStringLength,
    ];
    for error in errors {
        lines.push(format!("{error} {error:?}"));
    }
    lines.join("\n")
}
