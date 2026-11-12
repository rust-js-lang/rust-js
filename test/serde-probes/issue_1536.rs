// #1536, its comment 6828038118, the flatten probe: `#[serde(with)]`, `serialize_with` and `deserialize_with` are silently ignored on a variant, a newtype struct's field, a newtype variant's field and a `transparent` struct's field: `{"A":7}` for `{"A":"7"}`
use serde::ser::SerializeStruct;
use serde::{Serialize, Serializer};
#[derive(Serialize)]
struct Payload {
    source: u32,
}
fn wire<S: Serializer>(p: &Payload, s: S) -> Result<S::Ok, S::Error> {
    let mut st = s.serialize_struct("Payload", 1)?;
    st.serialize_field("wire", &p.source)?;
    st.end()
}
#[derive(Serialize)]
struct Outer {
    #[serde(flatten, serialize_with = "wire")]
    payload: Payload,
}
pub fn main() {
    println!(
        "{:?}",
        serde_json::to_string(&Outer {
            payload: Payload { source: 7 }
        })
    );
}
