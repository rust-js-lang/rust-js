// #1548, its repro: An internally tagged payload ignores its hand-written Serialize impl and exposes its Rust field name
use serde::ser::SerializeStruct;
use serde::{Serialize, Serializer};
struct Payload {
    source: u32,
}
impl Serialize for Payload {
    fn serialize<S: Serializer>(&self, s: S) -> Result<S::Ok, S::Error> {
        let mut st = s.serialize_struct("Payload", 1)?;
        st.serialize_field("wire", &self.source)?;
        st.end()
    }
}
#[derive(Serialize)]
#[serde(tag = "kind")]
enum Message {
    Data(Payload),
}
pub fn main() {
    println!("{:?}", serde_json::to_string(&Message::Data(Payload { source: 7 })));
}
