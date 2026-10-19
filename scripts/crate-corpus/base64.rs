use std::io::Write;

use base64::Engine;
use base64::display::Base64Display;
use base64::engine::general_purpose::{STANDARD, STANDARD_NO_PAD, URL_SAFE_NO_PAD};
use base64::write::EncoderStringWriter;

pub fn report() -> String {
    let mut lines = Vec::new();
    for text in ["", "f", "fo", "foo", "foob", "hello, wörld?", "\u{1F600}~~"] {
        let bytes = text.as_bytes();
        lines.push(format!(
            "{text:?}: {} {} {}",
            STANDARD.encode(bytes),
            STANDARD_NO_PAD.encode(bytes),
            URL_SAFE_NO_PAD.encode(bytes)
        ));
    }
    for encoded in ["aGVsbG8=", "aGVsbG8", "aGVs bG8=", "aGVsbG8==", "_-8"] {
        let decoded = STANDARD
            .decode(encoded)
            .map(|b| String::from_utf8_lossy(&b).into_owned());
        let url = URL_SAFE_NO_PAD.decode(encoded).map(|b| b.len());
        lines.push(format!("{encoded:?}: {decoded:?} {url:?}"));
    }
    lines.push(format!("[{}]", Base64Display::new(b"display me", &STANDARD)));
    let mut writer = EncoderStringWriter::new(&STANDARD);
    writer.write_all(b"streamed ").unwrap();
    writer.write_all(b"in parts").unwrap();
    lines.push(writer.into_inner());
    let mut buf = String::from("prefix:");
    STANDARD.encode_string(b"appended", &mut buf);
    lines.push(buf);
    lines.join("\n")
}
