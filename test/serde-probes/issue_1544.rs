// #1544, its repro: A hand-written serde Visitor runs `expecting` on successful reads, introducing side effects and possible panics
use serde::de::{Error, Visitor};
use serde::{Deserialize, Deserializer};
use std::fmt;
#[derive(Debug)]
struct Answer(bool);
struct V;
impl<'de> Visitor<'de> for V {
    type Value = Answer;
    fn expecting(&self, f: &mut fmt::Formatter) -> fmt::Result {
        println!("expecting ran");
        f.write_str("boolean")
    }
    fn visit_bool<E: Error>(self, b: bool) -> Result<Answer, E> {
        Ok(Answer(b))
    }
}
impl<'de> Deserialize<'de> for Answer {
    fn deserialize<D: Deserializer<'de>>(d: D) -> Result<Self, D::Error> {
        d.deserialize_bool(V)
    }
}
pub fn main() {
    println!("{:?}", serde_json::from_str::<Answer>("true"));
}
