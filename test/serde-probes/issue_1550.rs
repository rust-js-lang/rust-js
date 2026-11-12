// #1550, its repro: Serde codec lookup ignores generic arguments: Wrap<String> serializes with Wrap<u32>'s impl
use serde::{Serialize, Serializer};
struct Wrap<T>(T);
impl Serialize for Wrap<u32> {
    fn serialize<S: Serializer>(&self, s: S) -> Result<S::Ok, S::Error> {
        s.serialize_str("number impl")
    }
}
impl Serialize for Wrap<String> {
    fn serialize<S: Serializer>(&self, s: S) -> Result<S::Ok, S::Error> {
        s.serialize_str("string impl")
    }
}
pub fn main() {
    println!("{:?}", serde_json::to_string(&Wrap(7u32)));
    println!("{:?}", serde_json::to_string(&Wrap("hello".to_string())));
}
