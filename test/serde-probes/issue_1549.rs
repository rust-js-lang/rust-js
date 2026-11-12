// #1549, its repro: A custom option Visitor crashes under `from_value`: `$jsonVisitOption` assumes a text reader
use serde::de::{Error, Visitor};
use serde::{Deserialize, Deserializer};
use std::fmt;
#[derive(Debug)]
struct Answer(u32);
struct V;
impl<'de> Visitor<'de> for V {
    type Value = Answer;
    fn expecting(&self, f: &mut fmt::Formatter) -> fmt::Result {
        f.write_str("option")
    }
    fn visit_none<E: Error>(self) -> Result<Answer, E> {
        Ok(Answer(0))
    }
    fn visit_some<D: Deserializer<'de>>(self, d: D) -> Result<Answer, D::Error> {
        u32::deserialize(d).map(Answer)
    }
}
impl<'de> Deserialize<'de> for Answer {
    fn deserialize<D: Deserializer<'de>>(d: D) -> Result<Self, D::Error> {
        d.deserialize_option(V)
    }
}
pub fn main() {
    println!("{:?}", serde_json::from_str::<Answer>("null"));
    println!("{:?}", serde_json::from_value::<Answer>(serde_json::Value::Null));
}
