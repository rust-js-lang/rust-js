// #1545, its repro: A custom serializer cannot catch a nested serialization Err: generated JS removes its Err arm
use serde::ser::{Error, SerializeSeq};
use serde::{Serialize, Serializer};
struct Bad;
struct Outer;
impl Serialize for Bad {
    fn serialize<S: Serializer>(&self, _s: S) -> Result<S::Ok, S::Error> {
        Err(S::Error::custom("inner"))
    }
}
impl Serialize for Outer {
    fn serialize<S: Serializer>(&self, s: S) -> Result<S::Ok, S::Error> {
        let mut seq = s.serialize_seq(Some(1))?;
        match seq.serialize_element(&Bad) {
            Ok(()) => seq.end(),
            Err(_) => Err(S::Error::custom("caught")),
        }
    }
}
pub fn main() {
    println!("{:?}", serde_json::to_string(&Outer));
    println!("{:?}", serde_json::to_value(Outer));
}
