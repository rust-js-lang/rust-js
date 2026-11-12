// #1546, its repro: `from_value` calls `visit_str` instead of a custom Visitor's `visit_string`, changing the decoded value
use serde::de::{Error, Visitor};
use serde::{Deserialize, Deserializer};
use std::fmt;
#[derive(Debug)]
struct Answer(String);
struct V;
impl<'de> Visitor<'de> for V {
    type Value = Answer;
    fn expecting(&self, f: &mut fmt::Formatter) -> fmt::Result {
        f.write_str("string")
    }
    fn visit_str<E: Error>(self, s: &str) -> Result<Answer, E> {
        Ok(Answer(format!("str:{s}")))
    }
    fn visit_string<E: Error>(self, s: String) -> Result<Answer, E> {
        Ok(Answer(format!("owned:{s}")))
    }
}
impl<'de> Deserialize<'de> for Answer {
    fn deserialize<D: Deserializer<'de>>(d: D) -> Result<Self, D::Error> {
        d.deserialize_string(V)
    }
}
pub fn main() {
    println!("{:?}", serde_json::from_str::<Answer>("\"hi\""));
    println!(
        "{:?}",
        serde_json::from_value::<Answer>(serde_json::Value::String("hi".into()))
    );
}
