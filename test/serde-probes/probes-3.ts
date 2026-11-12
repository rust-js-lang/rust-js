import { writeFileSync } from "node:fs";
const head = String.raw`#![allow(unused, dead_code)]
use serde::de::DeserializeOwned;
use serde::{Deserialize, Deserializer, Serialize, Serializer};
use serde_json::{json, Map, Number, Value};
use std::collections::{BTreeMap, HashMap};
use std::fmt::Debug;
use std::num::{NonZeroU32, NonZeroU8, NonZeroI64};

fn w<T: Serialize>(value: &T) -> String {
    match serde_json::to_string(value) {
        Ok(text) => text,
        Err(error) => format!("ERR {error}"),
    }
}
fn r<T: DeserializeOwned + Debug>(text: &str) -> String {
    match serde_json::from_str::<T>(text) {
        Ok(value) => format!("ok {value:?}"),
        Err(e) => format!("err {e}"),
    }
}
fn up<S: Serializer>(n: &i32, s: S) -> Result<S::Ok, S::Error> { s.serialize_str("up") }
fn down<'de, D: Deserializer<'de>>(d: D) -> Result<i32, D::Error> { Ok(String::deserialize(d)?.len() as i32) }
mod txt {
    use serde::{Deserialize, Deserializer, Serializer};
    pub fn serialize<S: Serializer>(n: &i32, s: S) -> Result<S::Ok, S::Error> { s.serialize_str(&n.to_string()) }
    pub fn deserialize<'de, D: Deserializer<'de>>(d: D) -> Result<i32, D::Error> { String::deserialize(d)?.parse().map_err(serde::de::Error::custom) }
}
#[derive(Serialize, Deserialize, Debug)] struct Body { n: i32 }
`;
const P: Record<string, string> = {};
const R = String.raw;
const lines = [
  R`#[derive(Serialize, Debug)] enum E { #[serde(serialize_with = "up")] A(i32), B(i32) } println!("{} {}", w(&E::A(1)), w(&E::B(1)));`,
  R`#[derive(Deserialize, Debug)] enum E { #[serde(deserialize_with = "down")] A(i32), B(i32) } println!("{} {}", r::<E>(r#"{"A":"four"}"#), r::<E>(r#"{"A":4}"#));`,
  R`#[derive(Serialize, Deserialize, Debug)] enum E { #[serde(with = "txt")] A(i32), B(i32) } println!("{} {} {}", w(&E::A(7)), r::<E>(r#"{"A":"7"}"#), r::<E>(r#"{"A":7}"#));`,
  R`#[derive(Serialize, Deserialize, Debug)] #[serde(tag = "t", content = "c")] enum E { #[serde(with = "txt")] A(i32) } println!("{} {}", w(&E::A(7)), r::<E>(r#"{"t":"A","c":"7"}"#));`,
  R`#[derive(Serialize, Deserialize, Debug)] #[serde(untagged)] enum E { #[serde(with = "txt")] A(i32), B(bool) } println!("{} {} {}", w(&E::A(7)), r::<E>(r#""7""#), r::<E>("7"));`,
  R`#[derive(Serialize, Deserialize, Debug)] enum E { A(#[serde(with = "txt")] i32), T(i32, #[serde(with = "txt")] i32), S { #[serde(with = "txt")] a: i32 } } println!("{} {} {} {}", w(&E::A(7)), w(&E::T(1, 2)), w(&E::S { a: 3 }), r::<E>(r#"{"T":[1,"2"]}"#));`,
  R`#[derive(Serialize, Deserialize, Debug)] struct T(#[serde(with = "txt")] i32, i32); #[derive(Serialize, Deserialize, Debug)] struct N(#[serde(with = "txt")] i32); println!("{} {} {} {}", w(&T(1, 2)), w(&N(3)), r::<T>(r#"["1",2]"#), r::<N>(r#""3""#));`,
  R`println!("{}", r::<Box<i32>>("5"));`,
  R`println!("{}", r::<NonZeroU32>("0"));`,
  R`println!("{} {}", r::<NonZeroU32>("7"), r::<NonZeroU8>("256"));`,
  R`println!("{} {}", r::<Option<NonZeroU32>>("0"), r::<Vec<NonZeroI64>>("[1,-1]"));`,
  R`#[derive(Serialize, Deserialize, Debug)] struct S { id: NonZeroU32, o: Option<NonZeroU32> } println!("{} {}", w(&S { id: NonZeroU32::new(3).unwrap(), o: None }), r::<S>(r#"{"id":4,"o":5}"#));`,
  R`#[derive(Serialize, Deserialize, Debug)] struct In { #[serde(flatten)] m: BTreeMap<String, i32> } #[derive(Serialize, Deserialize, Debug)] #[serde(tag = "t")] enum E { A(In) } println!("{} {}", w(&E::A(In { m: BTreeMap::from([("k".to_string(), 1)]) })), r::<E>(r#"{"t":"A","k":1}"#));`,
  R`#[derive(Serialize, Deserialize, Debug)] #[serde(transparent)] struct Tr(Body); #[derive(Serialize, Deserialize, Debug)] #[serde(tag = "t")] enum E { A(Tr), B(Box<Body>) } println!("{} {} {}", w(&E::A(Tr(Body { n: 1 }))), w(&E::B(Box::new(Body { n: 2 }))), r::<E>(r#"{"t":"B","n":3}"#));`,
  R`#[derive(Serialize, Deserialize, Debug)] struct Has { m: HashMap<String, i32> } #[derive(Serialize, Deserialize, Debug)] #[serde(tag = "t")] enum E { A(Has) } println!("{} {}", w(&E::A(Has { m: HashMap::from([("k".to_string(), 1)]) })), r::<E>(r#"{"t":"A","m":{"k":1}}"#));`,
  R`#[derive(Serialize, Deserialize, Debug)] #[serde(tag = "t")] enum E { A(HashMap<String, i32>) } println!("{}", r::<E>(r#"{"t":"A","k":1}"#));`,
  R`#[derive(Serialize, Deserialize, Debug)] #[serde(tag = "t")] enum E { A(Vec<i32>) } println!("{}", w(&E::A(vec![1])));`,
  R`#[derive(Serialize, Deserialize, Debug)] #[serde(tag = "t")] enum E { A(String) } println!("{}", w(&E::A("s".into())));`,
  R`#[derive(Serialize, Deserialize, Debug)] #[serde(tag = "t")] enum E { A(Map<String, Value>) } let mut m = Map::new(); m.insert("k".into(), json!(1)); println!("{}", w(&E::A(m)));`,
  R`#[derive(Serialize, Deserialize, Debug)] #[serde(crate = "serde")] struct S { a: i32 } println!("{} {}", w(&S { a: 1 }), r::<S>(r#"{"a":1}"#));`,
  R`use serde::de::IntoDeserializer; let d: serde::de::value::StrDeserializer<serde::de::value::Error> = "x".into_deserializer(); println!("{:?}", String::deserialize(d));`,
  R`let v = json!({"n": 5}); println!("{:?}", Body::deserialize(v).map_err(|e| e.to_string()));`,
  R`let v = json!("no"); println!("{:?}", Body::deserialize(&v).map_err(|e| e.to_string()));`,
  R`let b: Body = serde_json::from_value(json!({"n": 5})).unwrap(); println!("{b:?}");`,
  R`#[derive(Serialize, Deserialize, Debug)] struct S { #[serde(serialize_with = "up", deserialize_with = "down")] a: i32, #[serde(default, deserialize_with = "down")] b: i32 } println!("{} {} {}", w(&S { a: 1, b: 2 }), r::<S>(r#"{"a":"four"}"#), r::<S>(r#"{"a":"x","b":"yy"}"#));`,
  R`#[derive(Serialize, Deserialize, Debug)] struct S { #[serde(with = "txt")] a: i32, #[serde(skip_serializing_if = "Option::is_none", with = "opt")] o: Option<i32> } mod opt { use serde::{Deserialize, Deserializer, Serializer}; pub fn serialize<S: Serializer>(n: &Option<i32>, s: S) -> Result<S::Ok, S::Error> { s.serialize_i32(n.unwrap_or(-1)) } pub fn deserialize<'de, D: Deserializer<'de>>(d: D) -> Result<Option<i32>, D::Error> { Ok(Some(i32::deserialize(d)?)) } } println!("{} {} {}", w(&S { a: 1, o: None }), w(&S { a: 1, o: Some(2) }), r::<S>(r#"{"a":"1"}"#));`,
  R`#[derive(Serialize, Deserialize, Debug)] struct S { #[serde(flatten, with = "fl")] a: Body } mod fl { use serde::{Deserialize, Deserializer, Serialize, Serializer}; use super::Body; pub fn serialize<S: Serializer>(b: &Body, s: S) -> Result<S::Ok, S::Error> { b.serialize(s) } pub fn deserialize<'de, D: Deserializer<'de>>(d: D) -> Result<Body, D::Error> { Body::deserialize(d) } } println!("{} {}", w(&S { a: Body { n: 1 } }), r::<S>(r#"{"n":1}"#));`,
  R`#[derive(Serialize, Deserialize, Debug)] struct G<T: Default> { #[serde(skip)] a: T, b: i32 } println!("{} {}", w(&G { a: 1u8, b: 2 }), r::<G<String>>(r#"{"b":2}"#));`,
  R`#[derive(Serialize, Deserialize, Debug)] struct S { #[serde(skip_serializing_if = "skip_it")] a: i32 } fn skip_it(n: &i32) -> bool { println!("asked {n}"); *n > 1 } println!("{} {}", w(&S { a: 1 }), w(&S { a: 2 }));`,
  R`#[derive(Serialize, Deserialize, Debug)] struct S { a: i32, #[serde(default = "noisy")] b: i32, #[serde(default = "noisy")] c: i32 } fn noisy() -> i32 { println!("default made"); 7 } println!("{}", r::<S>(r#"{"a":1,"c":3}"#)); println!("{}", r::<S>(r#"{"a":"x"}"#));`,
  R`#[derive(Debug)] struct Loud(i32); impl<'de> Deserialize<'de> for Loud { fn deserialize<D: Deserializer<'de>>(d: D) -> Result<Loud, D::Error> { let n = i32::deserialize(d)?; println!("read {n}"); Ok(Loud(n)) } } #[derive(Deserialize, Debug)] struct S { a: Loud, b: Loud, c: Loud } println!("{}", r::<S>(r#"{"c":3,"a":1,"b":"x"}"#)); println!("{}", r::<S>(r#"{"b":2,"b":4,"a":1}"#)); println!("{}", r::<(Loud, Loud)>(r#"[1,2,3]"#));`,
  R`struct Loud(i32); impl Serialize for Loud { fn serialize<S: Serializer>(&self, s: S) -> Result<S::Ok, S::Error> { println!("write {}", self.0); if self.0 == 2 { Err(serde::ser::Error::custom("two")) } else { s.serialize_i32(self.0) } } } #[derive(Serialize)] struct S { a: Loud, b: Loud, c: Loud } println!("{}", w(&S { a: Loud(1), b: Loud(2), c: Loud(3) })); println!("{}", w(&vec![Loud(3), Loud(1)]));`,
];
lines.forEach((line, i) => { P[`more_${String(i).padStart(2, "0")}`] = `${head}\nfn main() {\n${line}\n}\n`; });
for (const [name, text] of Object.entries(P)) writeFileSync(`p/${name}.rs`, text);
console.log(Object.keys(P).length);
