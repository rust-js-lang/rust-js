// Second batch: one program per statement, for what the first batch's refusals hid.
import { writeFileSync, mkdirSync } from "node:fs";
const head = String.raw`#![allow(unused, dead_code)]
use serde::de::DeserializeOwned;
use serde::{Deserialize, Serialize};
use serde_json::{json, Map, Number, Value};
use std::borrow::Cow;
use std::collections::{BTreeMap, BTreeSet, HashMap, HashSet, VecDeque};
use std::fmt::Debug;

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
`;
const P: Record<string, string> = {};
const R = String.raw;
const each = (name: string, items: string, prelude: string, lines: string[]) => {
  lines.forEach((line, i) => { P[`${name}_${String(i).padStart(2, "0")}`] = `${head}${items}\nfn main() {\n${prelude}\n${line}\n}\n`; });
};
const P_ = R`#[derive(Serialize, Deserialize, Debug, Clone, PartialEq)] struct P { x: i32 }`;
each("bytes", P_, "", [
  R`println!("{:?}", serde_json::to_vec(&P { x: 1 }).unwrap());`,
  R`println!("{}", String::from_utf8(serde_json::to_vec_pretty(&P { x: 1 }).unwrap()).unwrap());`,
  R`let mut out: Vec<u8> = Vec::new(); serde_json::to_writer(&mut out, &vec![1, 2]).unwrap(); println!("{}", String::from_utf8(out).unwrap());`,
  R`let mut out: Vec<u8> = Vec::new(); serde_json::to_writer_pretty(&mut out, &vec![1, 2]).unwrap(); println!("{}", String::from_utf8(out).unwrap());`,
  R`println!("{:?}", serde_json::from_slice::<P>(b"{\"x\":5}").map_err(|e| e.to_string()));`,
  R`println!("{:?}", serde_json::from_slice::<P>(&[0xff, 0xfe]).map_err(|e| e.to_string()));`,
  R`println!("{:?}", serde_json::from_slice::<String>(b"\"\xff\"").map_err(|e| e.to_string()));`,
  R`println!("{:?}", serde_json::from_reader::<_, P>(&b"{\"x\":6}"[..]).map_err(|e| e.to_string()));`,
  R`let text = String::from("{\"x\":7}"); println!("{:?}", serde_json::from_str::<P>(text.as_str()).map_err(|e| e.to_string()));`,
  R`println!("{}", serde_json::to_string(&&&P { x: 1 }).unwrap());`,
]);
each("std", "", "", [
  R`println!("{} {}", w(&BTreeSet::from([3, 1, 2])), w(&VecDeque::from([1, 2])));`,
  R`println!("{} {}", w(&Box::new(5)), w(&Box::new("rc")));`,
  R`println!("{}", w(&Cow::Borrowed("cow")));`,
  R`println!("{}", w(&std::time::Duration::new(3, 5)));`,
  R`println!("{}", w(&(1..4)));`,
  R`println!("{}", w(&(1..=4)));`,
  R`println!("{} {}", w(&Ok::<i32, String>(1)), w(&Err::<i32, String>("e".into())));`,
  R`println!("{}", w(&std::cell::Cell::new(1)));`,
  R`println!("{}", w(&std::cell::RefCell::new(vec![1])));`,
  R`println!("{}", w(&std::num::Wrapping(7u8)));`,
  R`println!("{}", w(&std::cmp::Reverse(3)));`,
  R`println!("{}", w(&std::path::PathBuf::from("a/b")));`,
  R`println!("{}", w(&std::num::NonZeroU32::new(9)));`,
  R`println!("{} {}", r::<BTreeSet<i32>>("[3,1,3]"), r::<VecDeque<i32>>("[1,2]"));`,
  R`println!("{} {}", r::<Result<i32, String>>(r#"{"Ok":1}"#), r::<Result<i32, String>>(r#"{"Err":"e"}"#));`,
  R`println!("{}", r::<std::time::Duration>(r#"{"secs":3,"nanos":5}"#));`,
  R`println!("{}", r::<std::ops::Range<i32>>(r#"{"start":1,"end":4}"#));`,
  R`println!("{} {}", r::<Box<i32>>("5"), r::<std::num::NonZeroU32>("0"));`,
  R`println!("{}", r::<Cow<str>>("\"c\""));`,
  R`println!("{} {}", r::<Box<str>>("\"c\""), r::<Box<[i32]>>("[1]"));`,
  R`println!("{}", w(&std::sync::Mutex::new(3)));`,
  R`println!("{}", w(&std::net::Ipv4Addr::new(127, 0, 0, 1)));`,
  R`println!("{}", w(&std::time::UNIX_EPOCH));`,
  R`println!("{} {}", w(&[1u8, 2, 3][..]), w(&"str"[1..]));`,
  R`println!("{}", w(&std::collections::LinkedList::from([1, 2])));`,
  R`println!("{}", w(&std::collections::BinaryHeap::from([1])));`,
  R`println!("{} {}", w(&(1, 2, 3, 4, 5, 6, 7, 8, 9, 10, 11, 12)), w(&[0u8; 3]));`,
  R`println!("{} {}", w(&std::marker::PhantomData::<i32>), r::<std::marker::PhantomData<i32>>("null"));`,
  R`println!("{}", w(&std::ops::Bound::Included(1)));`,
  R`println!("{} {} {}", w(&1usize), w(&-1isize), r::<usize>("4294967296"));`,
]);
each("ienum", R`#[derive(Serialize, Deserialize, Debug)] struct Body { n: i32 }
#[derive(Serialize, Deserialize, Debug)] #[serde(tag = "type")] enum E { Unit, Struct { a: i32 }, New(Body), Empty {} }
#[derive(Serialize, Deserialize, Debug)] #[serde(tag = "type")] enum M { Map(BTreeMap<String, i32>) }
#[derive(Serialize, Deserialize, Debug)] #[serde(tag = "type")] enum H { Map(HashMap<String, i32>) }
#[derive(Serialize, Deserialize, Debug)] #[serde(tag = "type")] enum N { Int(i32), Val(Value), Opt(Option<Body>), Unit(()), Inner(E) }
#[derive(Serialize, Deserialize, Debug)] #[serde(tag = "type")] enum Big { S { u: u64, i: i64, f: f64, s: String, o: Option<u64>, v: Vec<u64> } }`, "", [
  R`let all = vec![E::Unit, E::Struct { a: 1 }, E::New(Body { n: 2 }), E::Empty {}]; let text = w(&all); println!("{text}"); println!("{}", r::<Vec<E>>(&text));`,
  R`for s in [r#"{"a":1,"type":"Struct"}"#, r#"{"type":"Unit","extra":1}"#, r#"{"a":1}"#, r#"{"type":"Nope"}"#, r#"{"type":1}"#, r#""Unit""#, r#"["Unit"]"#, r#"{"type":"Struct"}"#, r#"{"type":"New","n":2,"x":1}"#, r#"{"type":"Struct","a":1,"type":"Unit"}"#, r#"["Struct", 1]"#, r#"["Unit"]"#] { println!("{s} => {}", r::<E>(s)); }`,
  R`println!("{}", w(&M::Map(BTreeMap::from([("k".to_string(), 3)]))));`,
  R`println!("{}", r::<M>(r#"{"k":3,"type":"Map","j":4}"#));`,
  R`println!("{}", w(&H::Map(HashMap::from([("k".to_string(), 3)]))));`,
  R`println!("{}", w(&N::Int(1)));`,
  R`println!("{} {}", w(&N::Val(json!({"a": 1}))), w(&N::Val(json!([1]))));`,
  R`println!("{} {}", w(&N::Opt(None)), w(&N::Opt(Some(Body { n: 1 }))));`,
  R`println!("{}", w(&N::Unit(())));`,
  R`println!("{}", w(&N::Inner(E::Struct { a: 1 })));`,
  R`println!("{} {}", r::<N>(r#"{"type":"Val","a":1}"#), r::<N>(r#"{"type":"Opt","n":1}"#));`,
  R`println!("{} {}", r::<N>(r#"{"type":"Int"}"#), r::<N>(r#"{"type":"Unit"}"#));`,
  R`println!("{}", r::<Big>(r#"{"u":18446744073709551615,"i":-9223372036854775808,"f":1.0,"s":"😀","o":9007199254740993,"v":[9007199254740993,1],"type":"S"}"#));`,
  R`println!("{}", r::<Big>(r#"{"u":1.0,"i":1,"f":1,"s":"","o":null,"v":[],"type":"S"}"#));`,
  R`println!("{}", r::<Big>(r#"{"u":1,"i":1,"f":1e400,"s":"","o":null,"v":[],"type":"S"}"#));`,
  R`println!("{}", r::<Big>("{\"u\":1,\n\"i\":true,\"f\":1,\"s\":\"\",\"o\":null,\"v\":[],\"type\":\"S\"}"));`,
]);
each("errapi", R`#[derive(Deserialize, Debug)] struct P { x: i32 }`, R`let e = serde_json::from_str::<P>("{\n \"x\": \"s\"}").unwrap_err();`, [
  R`println!("{e}");`,
  R`println!("{e:?}");`,
  R`println!("{}", e.line());`,
  R`println!("{}", e.column());`,
  R`println!("{:?}", e.classify());`,
  R`println!("{} {} {} {}", e.is_syntax(), e.is_data(), e.is_eof(), e.is_io());`,
  R`let b: Box<dyn std::error::Error> = Box::new(e); println!("{b} | {}", b.source().is_none());`,
  R`let io: std::io::Error = e.into(); println!("{:?} {io}", io.kind());`,
  R`println!("{}", e.to_string().len());`,
  R`fn f() -> Result<P, serde_json::Error> { let p: P = serde_json::from_str("{")?; Ok(p) } println!("{:?}", f().map_err(|e| e.to_string()));`,
  R`fn f() -> Result<P, Box<dyn std::error::Error>> { let p: P = serde_json::from_str("{")?; Ok(p) } println!("{:?}", f().map_err(|e| e.to_string()));`,
  R`let u = serde_json::from_str::<Value>("[1,2").unwrap_err(); println!("{u} {:?}", u.to_string());`,
  R`let u = serde_json::from_str::<String>("\"é\" x").unwrap_err(); println!("{u}");`,
  R`let u = serde_json::from_str::<Vec<String>>("[\"😀😀\", 1]").unwrap_err(); println!("{u}");`,
  R`let u = serde_json::from_str::<Vec<String>>("[\"a\",\n\"é😀\", \u{e9} ]").unwrap_err(); println!("{u}");`,
  R`let u = serde_json::from_str::<BTreeMap<String, i32>>("{\"ключ\": \"x\"}").unwrap_err(); println!("{u}");`,
  R`let u = serde_json::from_str::<P>("{\"x\": 1}\u{2028}").unwrap_err(); println!("{u}");`,
  R`let u = serde_json::from_str::<P>("\t{\"x\":\t\"é\"}").unwrap_err(); println!("{u}");`,
]);
each("mapkey", R`#[derive(Serialize, Deserialize, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)] enum K { A, B }
#[derive(Serialize, Deserialize, Debug, PartialEq, Eq, PartialOrd, Ord)] struct Id(u32);
#[derive(Serialize, Deserialize, Debug, PartialEq, Eq, PartialOrd, Ord)] #[serde(transparent)] struct Name(String);`, "", [
  R`println!("{}", w(&BTreeMap::from([("b", 1), ("a", 2)])));`,
  R`println!("{}", w(&BTreeMap::from([(2, "x"), (-1, "y")])));`,
  R`println!("{}", w(&BTreeMap::from([(true, 1), (false, 0)])));`,
  R`println!("{}", w(&BTreeMap::from([('z', 1), ('a', 0)])));`,
  R`println!("{}", w(&BTreeMap::from([(K::B, 1), (K::A, 0)])));`,
  R`println!("{}", w(&BTreeMap::from([(u64::MAX, 1)])));`,
  R`println!("{}", w(&BTreeMap::from([((1, 2), 1)])));`,
  R`println!("{}", w(&BTreeMap::from([(Some(1), 1)])));`,
  R`println!("{}", w(&BTreeMap::from([(vec![1], 1)])));`,
  R`println!("{}", w(&BTreeMap::from([(Id(4), 1)])));`,
  R`println!("{}", w(&BTreeMap::from([(Name("n".into()), 1)])));`,
  R`println!("{}", r::<BTreeMap<K, i32>>(r#"{"B":1,"A":0}"#));`,
  R`println!("{}", r::<BTreeMap<Id, i32>>(r#"{"4":1}"#));`,
  R`println!("{}", r::<BTreeMap<Name, i32>>(r#"{"n":1}"#));`,
  R`println!("{}", r::<BTreeMap<u64, i32>>(r#"{"18446744073709551615":1,"007":2}"#));`,
  R`println!("{}", r::<BTreeMap<i8, i32>>(r#"{"-128":1,"+1":2}"#));`,
  R`println!("{}", r::<BTreeMap<String, i32>>(r#"{"__proto__":1,"constructor":2,"toString":3}"#));`,
  R`println!("{}", w(&serde_json::from_str::<Value>(r#"{"__proto__":1,"constructor":{"prototype":2},"hasOwnProperty":3}"#).unwrap()));`,
  R`#[derive(Deserialize, Debug)] struct S { a: i32 } println!("{}", r::<S>(r#"{"__proto__":{"a":5},"a":1}"#));`,
  R`#[derive(Deserialize, Debug, Default)] #[serde(default)] struct S { to_string: i32, constructor: i32 } println!("{}", r::<S>(r#"{}"#));`,
  R`let v: Value = serde_json::from_str(r#"{"__proto__":{"x":1}}"#).unwrap(); println!("{} {} {}", v["x"], v["__proto__"]["x"], v.as_object().unwrap().len());`,
  R`let m: HashMap<String, i32> = serde_json::from_str(r#"{"__proto__":7}"#).unwrap(); println!("{:?} {}", m.get("__proto__"), m.len());`,
]);
each("big", "", "", [
  R`println!("{}", w(&i128::MAX));`,
  R`println!("{}", w(&u128::MAX));`,
  R`println!("{}", r::<u128>("340282366920938463463374607431768211455"));`,
  R`println!("{}", r::<i128>("-5"));`,
  R`println!("{} {} {}", r::<f32>("0.1"), r::<f32>("1e39"), r::<f32>("16777217"));`,
  R`println!("{} {}", r::<f32>("3.4028236e38"), r::<f32>("1e-46"));`,
  R`println!("{} {}", w(&(0.1f32 as f64)), w(&(1.0e7f32)));`,
  R`println!("{} {} {}", w(&1e16f64), w(&1.0e-7f64), w(&123456789.125f64));`,
  R`println!("{} {}", r::<u64>("9007199254740993"), r::<i64>("-9007199254740993"));`,
  R`println!("{} {}", r::<Vec<u64>>("[9007199254740993, 18446744073709551615]"), r::<(i64, u64)>("[-9223372036854775807, 9223372036854775808]"));`,
  R`println!("{}", r::<u64>("1e3"));`,
  R`println!("{}", r::<f64>("18446744073709551615"));`,
  R`println!("{}", r::<f64>("123456789012345678901234567890e-10"));`,
  R`println!("{}", r::<i64>("9223372036854775808"));`,
]);
each("stream", "", "", [
  R`let stream = serde_json::Deserializer::from_str("1 [2] {\"a\":3}").into_iter::<Value>(); for item in stream { match item { Ok(v) => print!("{v} / "), Err(e) => print!("err {e} / ") } } println!();`,
  R`let mut de = serde_json::Deserializer::from_str("[1,2] trailing"); let v = Vec::<i32>::deserialize(&mut de); println!("{v:?} {:?}", de.end().map_err(|e| e.to_string()));`,
  R`let mut de = serde_json::Deserializer::from_str("[1,2]"); let v = Vec::<i32>::deserialize(&mut de); println!("{v:?}");`,
  R`let v = i32::deserialize(json!(5)); println!("{v:?}");`,
  R`let v = <Vec<String>>::deserialize(&json!(["a"])); println!("{v:?}");`,
  R`let mut ser = serde_json::Serializer::new(Vec::new()); vec![1, 2].serialize(&mut ser).unwrap(); println!("{}", String::from_utf8(ser.into_inner()).unwrap());`,
  R`let raw: Box<serde_json::value::RawValue> = serde_json::from_str("{\"a\": [1, 2]}").unwrap(); println!("{}", raw.get());`,
]);
const V = R`let v: Value = serde_json::from_str(r#"{"a":[1,2,{"b":"x"}],"n":-5,"u":18446744073709551615,"f":1.5,"s":"str","t":true,"z":null,"o":{"k":1}}"#).unwrap();`;
each("vacc", "", V, [
  R`println!("{} {} {} {}", v["a"][0], v["a"][2]["b"], v["missing"], v["a"][9]);`,
  R`println!("{:?} {:?} {:?} {:?}", v["n"].as_i64(), v["n"].as_u64(), v["n"].as_f64(), v["u"].as_i64());`,
  R`println!("{:?} {:?} {:?} {:?}", v["u"].as_u64(), v["f"].as_i64(), v["f"].as_f64(), v["s"].as_str());`,
  R`println!("{:?} {:?}", v["t"].as_bool(), v["a"].as_array().map(|a| a.len()));`,
  R`println!("{:?} {:?}", v["z"].as_null(), v["t"].as_null());`,
  R`println!("{} {} {} {} {} {} {}", v["n"].is_number(), v["n"].is_i64(), v["u"].is_u64(), v["f"].is_f64(), v["s"].is_string(), v["z"].is_null(), v["o"].is_object());`,
  R`println!("{} {} {}", v["t"].is_boolean(), v["a"].is_array(), v["u"].as_f64().unwrap());`,
  R`println!("{:?} {:?} {:?}", v.get("a").and_then(|a| a.get(1)), v.get("nope"), v["a"].get(5));`,
  R`println!("{:?} {:?} {:?} {:?}", v.pointer("/a/2/b"), v.pointer("/o/k"), v.pointer("/a/x"), v.pointer(""));`,
  R`println!("{} {} {} {}", v["n"] == -5, v["s"] == "str", v["t"] == true, v["f"] == 1.5);`,
  R`println!("{:?}", v["o"].as_object().map(|o| o.keys().cloned().collect::<Vec<_>>()));`,
  R`println!("{:?}", v);`,
  R`println!("{:#?}", v["a"]);`,
  R`println!("{:#}", v["a"]);`,
  R`println!("{}", v.to_string().len());`,
  R`match &v["a"] { Value::Array(items) => println!("{}", items.len()), other => println!("other {other}") }`,
  R`match &v["n"] { Value::Number(n) => println!("{n} {:?} {}", n.as_i64(), n.is_f64()), other => println!("other {other}") }`,
  R`if let Value::Object(map) = &v { for (k, val) in map { print!("{k}={val};"); } println!(); }`,
  R`println!("{} {}", v == v.clone(), v["a"] == json!([1, 2, {"b": "x"}]));`,
  R`println!("{:?} {:?}", v["s"].as_str().map(str::len), v["s"].as_str().unwrap_or("none"));`,
  R`let o = v.as_object().unwrap(); println!("{} {:?} {}", o.contains_key("a"), o.get("n"), o.len());`,
  R`let items: Vec<i64> = v["a"].as_array().unwrap().iter().filter_map(Value::as_i64).collect(); println!("{items:?}");`,
  R`println!("{} {}", Value::default(), Value::Null == json!(null));`,
  R`println!("{} {} {} {}", Value::from(1), Value::from("s"), Value::from(vec![1, 2]), Value::from(1.5));`,
  R`println!("{} {}", Value::from(true), Value::from(String::from("x")));`,
  R`let n: Number = 5.into(); println!("{n} {:?} {:?}", Number::from_f64(1.0), Number::from_f64(f64::NAN));`,
  R`println!("{}", v["u"].as_u64().unwrap() - 1);`,
  R`println!("{}", v["o"]["k"].as_i64().unwrap_or(0) + 1);`,
]);
each("vmut", "", R`let mut v = json!({"a": 1, "b": [1, 2]});`, [
  R`v["b"] = json!([1, 2, 3]); println!("{v}");`,
  R`v["new"] = json!("changed"); println!("{v}");`,
  R`v["c"]["d"] = json!(true); println!("{v}");`,
  R`v["b"][0] = json!(null); println!("{v}");`,
  R`let taken = v["b"].take(); println!("{taken} {v}");`,
  R`if let Some(map) = v.as_object_mut() { map.insert("z".into(), json!(1)); map.remove("a"); } println!("{v}");`,
  R`v.as_object_mut().unwrap().insert("z".to_string(), Value::Null); println!("{v}");`,
  R`v["b"].as_array_mut().unwrap().push(json!({"k": "v"})); println!("{v}");`,
  R`if let Some(x) = v.get_mut("a") { *x = json!(2); } println!("{v}");`,
  R`*v.pointer_mut("/b/1").unwrap() = json!("p"); println!("{v}");`,
  R`let mut m = Map::new(); m.insert("k".to_string(), json!(1)); m.insert("a".to_string(), json!(2)); let o = Value::Object(m); println!("{o}");`,
  R`let mut m: Map<String, Value> = Map::new(); m.insert("b".into(), 1.into()); m.insert("a".into(), 2.into()); println!("{:?} {}", m.keys().collect::<Vec<_>>(), m.len());`,
  R`let c = v.clone(); v["a"] = json!(9); println!("{c} {v}");`,
  R`let arr = v["b"].as_array().unwrap().clone(); v["b"] = Value::Null; println!("{arr:?} {v}");`,
  R`let b = std::mem::take(&mut v["b"]); println!("{b} {v}");`,
  R`if let Value::Array(items) = &mut v["b"] { items.retain(|x| x != &json!(1)); } println!("{v}");`,
  R`let e = v.as_object_mut().unwrap().entry("n").or_insert(json!(0)); *e = json!(e.as_i64().unwrap() + 1); println!("{v}");`,
  R`v.as_object_mut().unwrap().retain(|k, _| k != "a"); println!("{v}");`,
  R`v[0] = json!(1);`,
  R`let mut n = json!(null); n["k"] = json!(1); println!("{n}");`,
]);
each("vconv", R`#[derive(Serialize, Deserialize, Debug)] struct P { x: i32, tags: Vec<String>, opt: Option<f64> }
#[derive(Serialize, Deserialize, Debug)] enum E { U, N(i32), S { a: i32 } }`, "", [
  R`let v = serde_json::to_value(P { x: 1, tags: vec!["a".into()], opt: Some(2.5) }).unwrap(); println!("{v}");`,
  R`println!("{:?}", serde_json::from_value::<P>(json!({"x": 1, "tags": ["t"], "opt": null})).map_err(|e| e.to_string()));`,
  R`println!("{:?}", serde_json::from_value::<P>(json!({"x": "no", "tags": []})).map_err(|e| e.to_string()));`,
  R`println!("{:?}", serde_json::from_value::<P>(json!({"tags": []})).map_err(|e| e.to_string()));`,
  R`println!("{:?}", serde_json::from_value::<Vec<i32>>(json!([1, 2.5])).map_err(|e| e.to_string()));`,
  R`println!("{:?}", serde_json::from_value::<BTreeMap<String, Value>>(json!({"k": [1]})).map_err(|e| e.to_string()));`,
  R`println!("{} {}", serde_json::to_value(u64::MAX).unwrap(), serde_json::to_value(f64::NAN).unwrap());`,
  R`println!("{} {}", serde_json::to_value(BTreeMap::from([(1, 2)])).unwrap(), serde_json::to_value((1, "a")).unwrap());`,
  R`println!("{:?}", serde_json::to_value(BTreeMap::from([((1, 2), 3)])).map_err(|e| e.to_string()));`,
  R`println!("{} {} {}", serde_json::to_value(E::U).unwrap(), serde_json::to_value(E::N(1)).unwrap(), serde_json::to_value(E::S { a: 1 }).unwrap());`,
  R`println!("{:?} {:?}", serde_json::from_value::<E>(json!("U")).map_err(|e| e.to_string()), serde_json::from_value::<E>(json!({"S": {"a": 1}})).map_err(|e| e.to_string()));`,
  R`println!("{:?} {:?}", serde_json::from_value::<E>(json!("X")).map_err(|e| e.to_string()), serde_json::from_value::<E>(json!({"N": "s"})).map_err(|e| e.to_string()));`,
  R`println!("{:?} {:?}", serde_json::from_value::<u8>(json!(300)).map_err(|e| e.to_string()), serde_json::from_value::<f64>(json!(1)).map_err(|e| e.to_string()));`,
  R`println!("{:?} {:?}", serde_json::from_value::<i32>(json!(1.0)).map_err(|e| e.to_string()), serde_json::from_value::<(i32, i32)>(json!([1])).map_err(|e| e.to_string()));`,
  R`let v: Value = serde_json::from_str("1.0").unwrap(); println!("{} {}", v, serde_json::to_value(1.0f32).unwrap());`,
  R`let p: P = serde_json::from_value(serde_json::to_value(P { x: 3, tags: vec![], opt: None }).unwrap()).unwrap(); println!("{p:?}");`,
  R`let v = json!({"x": 1, "tags": [], "opt": 1}); let p = P::deserialize(&v).unwrap(); println!("{p:?} {v}");`,
  R`println!("{}", serde_json::to_value(&json!([1, {"a": null}])).unwrap());`,
]);
each("attr", "", "", [
  R`#[derive(Serialize, Deserialize, Debug)] #[serde(tag = "type")] struct S { a: i32 } println!("{} {}", w(&S { a: 1 }), r::<S>(r#"{"type":"S","a":1}"#));`,
  R`#[derive(Serialize, Deserialize, Debug)] #[serde(rename = "Renamed")] struct S { a: i32 } println!("{} {}", w(&S { a: 1 }), r::<S>(r#"[1,2]"#));`,
  R`#[derive(Serialize, Deserialize, Debug)] #[serde(rename_all(serialize = "camelCase", deserialize = "SCREAMING_SNAKE_CASE"))] struct S { my_field: i32 } println!("{} {}", w(&S { my_field: 1 }), r::<S>(r#"{"MY_FIELD":1}"#));`,
  R`#[derive(Serialize, Deserialize, Debug)] struct T(i32, #[serde(default)] i32, #[serde(skip)] i32); println!("{} {} {}", w(&T(1, 2, 3)), r::<T>("[1]"), r::<T>("[1,2]"));`,
  R`#[derive(Serialize, Deserialize, Debug)] enum E { A(i32), #[serde(untagged)] B(String) } println!("{} {} {} {}", w(&E::A(1)), w(&E::B("b".into())), r::<E>(r#""x""#), r::<E>(r#"{"A":1}"#));`,
  R`fn up<S: serde::Serializer>(n: &i32, s: S) -> Result<S::Ok, S::Error> { s.serialize_str("up") } #[derive(Serialize)] enum E { #[serde(serialize_with = "up")] A(i32) } println!("{}", w(&E::A(1)));`,
  R`#[derive(Serialize, Deserialize, Debug)] #[serde(bound = "T: Serialize + DeserializeOwned")] struct S<T> { a: T } println!("{} {}", w(&S { a: 1 }), r::<S<i32>>(r#"{"a":1}"#));`,
  R`#[derive(Deserialize, Debug)] #[serde(expecting = "a point object")] struct S { a: i32 } println!("{} {}", r::<S>("5"), r::<S>("true"));`,
  R`#[derive(Deserialize, Debug)] #[serde(field_identifier, rename_all = "lowercase")] enum F { A, B } println!("{} {}", r::<F>(r#""a""#), r::<F>(r#""c""#));`,
  R`#[derive(Deserialize, Debug)] #[serde(variant_identifier)] enum F { A, B } println!("{} {} {}", r::<F>(r#""A""#), r::<F>("1"), r::<F>("5"));`,
  R`mod other { pub struct Remote { pub a: i32 } } #[derive(Serialize)] #[serde(remote = "other::Remote")] struct Def { a: i32 } #[derive(Serialize)] struct H { #[serde(with = "Def")] r: other::Remote } println!("{}", w(&H { r: other::Remote { a: 1 } }));`,
  R`#[derive(Serialize, Deserialize, Debug)] enum E { #[serde(rename(serialize = "out", deserialize = "in"))] A } println!("{} {} {}", w(&E::A), r::<E>(r#""in""#), r::<E>(r#""out""#));`,
  R`#[derive(Serialize, Deserialize, Debug)] enum E { A { #[serde(rename = "b")] a: i32, #[serde(skip_serializing_if = "Option::is_none", default)] c: Option<i32> } } println!("{} {}", w(&E::A { a: 1, c: None }), r::<E>(r#"{"A":{"b":2}}"#));`,
  R`#[derive(Serialize, Deserialize, Debug)] enum E { T(i32, #[serde(skip)] i32, String) } println!("{} {}", w(&E::T(1, 2, "s".into())), r::<E>(r#"{"T":[1,"s"]}"#));`,
  R`#[derive(Serialize, Deserialize, Debug)] enum E { O(Option<i32>), U(()), V(Vec<i32>), N(Box<E>) } println!("{} {} {} {}", w(&E::O(None)), w(&E::U(())), w(&E::N(Box::new(E::V(vec![])))), r::<E>(r#"{"O":null}"#));`,
  R`#[derive(Serialize, Deserialize, Debug)] #[serde(untagged)] enum E { A { #[serde(flatten)] m: BTreeMap<String, i32> }, B(Value) } println!("{} {} {}", w(&E::A { m: BTreeMap::from([("k".to_string(), 1)]) }), r::<E>(r#"{"k":1}"#), r::<E>(r#"{"k":"s"}"#));`,
  R`#[derive(Serialize, Deserialize, Debug)] #[serde(tag = "t", content = "c", deny_unknown_fields)] enum E { A(i32) } println!("{} {}", r::<E>(r#"{"t":"A","c":1,"x":2}"#), r::<E>(r#"{"c":1,"t":"A"}"#));`,
  R`#[derive(Serialize, Deserialize, Debug, Default)] struct S { #[serde(default, rename = "a-b")] a: i32, #[serde(alias = "c", default)] b: Vec<i32> } println!("{} {}", w(&S::default()), r::<S>(r#"{"c":[1],"a-b":2}"#));`,
  R`#[derive(Serialize, Deserialize, Debug)] struct S<'a> { a: &'a str } let text = String::from(r#"{"a":"plain"}"#); let s: S = serde_json::from_str(&text).unwrap(); println!("{s:?} {}", w(&s));`,
  R`#[derive(Deserialize, Debug)] struct S<'a> { a: &'a str } let text = String::from(r#"{"a":"a\nb"}"#); println!("{:?}", serde_json::from_str::<S>(&text).map_err(|e| e.to_string()));`,
  R`#[derive(Serialize, Deserialize, Debug)] struct W<T> { inner: T, list: Vec<T> } println!("{} {}", w(&W { inner: 1, list: vec![2] }), r::<W<String>>(r#"{"inner":"a","list":["b"]}"#));`,
  R`#[derive(Serialize, Deserialize, Debug)] struct Pd<T> { v: i32, #[serde(skip)] m: std::marker::PhantomData<T> } println!("{} {}", w(&Pd::<String> { v: 1, m: std::marker::PhantomData }), r::<Pd<String>>(r#"{"v":1}"#));`,
  R`#[derive(Serialize, Deserialize, Debug)] struct S { r#type: i32, r#fn: i32 } println!("{} {}", w(&S { r#type: 1, r#fn: 2 }), r::<S>(r#"{"type":1,"fn":2}"#));`,
  R`#[derive(Serialize, Deserialize, Debug)] struct S { #[serde(flatten)] a: BTreeMap<String, i32>, #[serde(flatten)] b: BTreeMap<String, Value> } println!("{}", r::<S>(r#"{"x":1,"y":"s"}"#));`,
]);
each("manual", R`use serde::ser::{SerializeMap, SerializeSeq, SerializeStruct, SerializeTuple, SerializeStructVariant};
use serde::de::{self, MapAccess, SeqAccess, Visitor};
use serde::{Deserializer, Serializer};
use std::fmt;
struct Pt { x: i32, y: i32 }
impl Serialize for Pt {
    fn serialize<S: Serializer>(&self, s: S) -> Result<S::Ok, S::Error> {
        let mut st = s.serialize_struct("Pt", 2)?;
        st.serialize_field("x", &self.x)?;
        st.serialize_field("why", &self.y)?;
        st.end()
    }
}
struct AsMap(Vec<(String, i32)>);
impl Serialize for AsMap {
    fn serialize<S: Serializer>(&self, s: S) -> Result<S::Ok, S::Error> {
        let mut m = s.serialize_map(Some(self.0.len()))?;
        for (k, v) in &self.0 { m.serialize_entry(k, v)?; }
        m.end()
    }
}
struct AsSeq(Vec<i32>);
impl Serialize for AsSeq {
    fn serialize<S: Serializer>(&self, s: S) -> Result<S::Ok, S::Error> {
        let mut q = s.serialize_seq(None)?;
        for v in &self.0 { q.serialize_element(&(v * 2))?; }
        q.end()
    }
}
struct Hex(u32);
impl Serialize for Hex {
    fn serialize<S: Serializer>(&self, s: S) -> Result<S::Ok, S::Error> { s.collect_str(&format_args!("{:#x}", self.0)) }
}
struct Fails;
impl Serialize for Fails {
    fn serialize<S: Serializer>(&self, _: S) -> Result<S::Ok, S::Error> { Err(serde::ser::Error::custom("never serializes")) }
}
#[derive(Debug)] struct Pair(i32, i32);
impl<'de> Deserialize<'de> for Pair {
    fn deserialize<D: Deserializer<'de>>(d: D) -> Result<Pair, D::Error> {
        struct V;
        impl<'de> Visitor<'de> for V {
            type Value = Pair;
            fn expecting(&self, f: &mut fmt::Formatter) -> fmt::Result { f.write_str("a pair as [a, b], {a, b} or \"a,b\"") }
            fn visit_seq<A: SeqAccess<'de>>(self, mut seq: A) -> Result<Pair, A::Error> {
                let a = seq.next_element()?.ok_or_else(|| de::Error::invalid_length(0, &self))?;
                let b = seq.next_element()?.ok_or_else(|| de::Error::invalid_length(1, &self))?;
                Ok(Pair(a, b))
            }
            fn visit_map<A: MapAccess<'de>>(self, mut map: A) -> Result<Pair, A::Error> {
                let (mut a, mut b) = (None, None);
                while let Some(key) = map.next_key::<String>()? {
                    match key.as_str() {
                        "a" => a = Some(map.next_value()?),
                        "b" => b = Some(map.next_value()?),
                        other => return Err(de::Error::unknown_field(other, &["a", "b"])),
                    }
                }
                Ok(Pair(a.ok_or_else(|| de::Error::missing_field("a"))?, b.ok_or_else(|| de::Error::missing_field("b"))?))
            }
            fn visit_str<E: de::Error>(self, s: &str) -> Result<Pair, E> {
                let (a, b) = s.split_once(',').ok_or_else(|| de::Error::invalid_value(de::Unexpected::Str(s), &self))?;
                Ok(Pair(a.parse().map_err(de::Error::custom)?, b.parse().map_err(de::Error::custom)?))
            }
        }
        d.deserialize_any(V)
    }
}`, "", [
  R`println!("{}", w(&Pt { x: 1, y: 2 }));`,
  R`println!("{}", w(&AsMap(vec![("b".into(), 1), ("a".into(), 2), ("b".into(), 3)])));`,
  R`println!("{}", w(&AsSeq(vec![1, 2])));`,
  R`println!("{}", w(&Hex(255)));`,
  R`println!("{} {}", w(&Fails), w(&vec![Some(Fails)]));`,
  R`println!("{:?}", serde_json::to_value(Fails).map_err(|e| e.to_string()));`,
  R`println!("{} {} {}", r::<Pair>("[1,2]"), r::<Pair>(r#"{"b":2,"a":1}"#), r::<Pair>(r#""3,4""#));`,
  R`println!("{} {} {}", r::<Pair>("[1]"), r::<Pair>(r#"{"a":1,"c":2}"#), r::<Pair>(r#""34""#));`,
  R`println!("{} {} {}", r::<Pair>("5"), r::<Pair>("null"), r::<Pair>(r#""x,1""#));`,
  R`println!("{}", r::<Vec<Pair>>("[[1,2],\n {\"a\":1}]"));`,
  R`println!("{}", serde_json::to_string_pretty(&(Pt { x: 1, y: 2 }, AsMap(vec![]), AsSeq(vec![]))).unwrap());`,
]);
mkdirSync("p", { recursive: true });
for (const [name, text] of Object.entries(P)) writeFileSync(`p/${name}.rs`, text.replaceAll("\\u00E9", String.fromCharCode(233)));
writeFileSync("names2.txt", Object.keys(P).join("\n") + "\n");
console.log(Object.keys(P).length);
