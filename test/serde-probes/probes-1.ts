// One-probe programs of serde / serde_json, each compared with native Rust.
import { writeFileSync, mkdirSync } from "node:fs";
const head = String.raw`#![allow(unused, dead_code)]
use serde::de::DeserializeOwned;
use serde::{Deserialize, Serialize};
use serde_json::{json, Value};
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
const fix = (s: string) => s.replaceAll("\\u00E9", String.fromCharCode(233));
const p = (name: string, items: string, body: string) => { P[name] = fix(`${head}${items}\nfn main() {\n${body}\n}\n`); };
const R = String.raw;

// A. numbers and primitives
p("num_ints", "", R`println!("{}", w(&(i8::MIN, i8::MAX, u8::MAX, i16::MIN, u16::MAX, i32::MIN, u32::MAX)));
println!("{}", w(&(i64::MIN, i64::MAX, u64::MAX, 0u64, -1i64)));
println!("{} {} {}", r::<u64>("18446744073709551615"), r::<i64>("-9223372036854775808"), r::<u64>("18446744073709551616"));
println!("{} {} {} {}", r::<u8>("255"), r::<u8>("256"), r::<i8>("-129"), r::<u32>("-1"));`);
p("num_i128", "", R`println!("{}", w(&(i128::MAX, u128::MAX, i128::MIN)));
println!("{} {}", r::<u128>("340282366920938463463374607431768211455"), r::<i128>("-5"));`);
p("num_f64_write", "", R`let xs = [0.0, -0.0, 1.0, 1.5, 1e21, 1e-7, f64::MAX, f64::MIN_POSITIVE, 5e-324, 0.1 + 0.2, 1e15, 1e16, 123456789012345680.0, 0.000001, 1e-5, 100.0, 1e300, 2.5e-10];
for x in xs { println!("{}", w(&x)); }`);
p("num_f32_write", "", R`let xs = [0.0f32, -0.0, 1.0, 1.5, 0.1, 16777216.0, 3.4028235e38, 1e-45, 0.3, 1e10, 1.1, 123456.79];
for x in xs { println!("{}", w(&x)); }`);
p("num_nan_inf", "", R`println!("{} {} {}", w(&f64::NAN), w(&f64::INFINITY), w(&f64::NEG_INFINITY));
println!("{} {}", w(&vec![1.0, f64::NAN]), w(&Some(f32::NAN)));
println!("{:?}", serde_json::Number::from_f64(f64::NAN).is_none());`);
p("num_read_f64", "", R`for s in ["1", "1.0", "1e3", "-0", "-0.0", "1E+2", "1e-2", "0.1e1", "123456789012345678901234567890", "1e308", "1e309", "-1e400", "4.9e-324", "1e-400", "0.30000000000000004", "9007199254740993"] {
    println!("{s} => {}", r::<f64>(s));
}`);
p("num_read_bad", "", R`for s in ["01", "+1", ".5", "1.", "0x10", "1e", "1e+", "-", "--1", "1.e1", "00", "1_000", "NaN", "Infinity", "1f", "0b1"] {
    println!("{s} => {}", r::<f64>(s));
}`);
p("num_read_int_from_float", "", R`for s in ["1.0", "1e2", "-1", "1.5", "3000000000", "\"1\"", "true", "null", "[]"] {
    println!("{s} => {} | {}", r::<i32>(s), r::<u64>(s));
}`);
p("num_value_roundtrip", "", R`for s in ["12345678901234567890", "123456789012345678901234567890", "-0", "1.0", "1e2", "1E-2", "0.1e1", "-9223372036854775808", "-9223372036854775809", "18446744073709551615", "1.0e0", "100", "1.10"] {
    let v: Result<Value, _> = serde_json::from_str(s);
    match v { Ok(v) => println!("{s} => {} {} {} {}", v, v.is_i64(), v.is_u64(), v.is_f64()), Err(e) => println!("{s} => err {e}") }
}`);
p("prim_bool_char_unit", "", R`println!("{} {} {} {} {} {}", w(&true), w(&'a'), w(&'\n'), w(&'é'), w(&'\u{1F600}'), w(&()));
println!("{} {} {} {}", r::<char>("\"a\""), r::<char>("\"ab\""), r::<char>("\"\""), r::<char>("\"\\u00e9\""));
println!("{} {} {} {}", r::<bool>("true"), r::<bool>("1"), r::<()>("null"), r::<()>("[]"));`);
p("str_write_escapes", "", R`let s = "quote\" slash\\ nl\n tab\t cr\r bs\u{8} ff\u{c} nul\u{0} del\u{7f} esc\u{1b} u2028\u{2028} u2029\u{2029} emoji😀 é / <tag>";
println!("{}", w(&s));
println!("{}", w(&String::from("\u{1}\u{1f}\u{80}\u{ffff}")));`);
p("str_read_escapes", "", R`for s in [r#""\u00e9""#, r#""\ud83d\ude00""#, r#""\ud800""#, r#""\ude00""#, r#""\ud800\u0041""#, r#""\x41""#, r#""\u12""#, r#""\/\b\f\n\r\t\"\\""#, r#""abc"#, "\"a\tb\"", "\"a\nb\"", r#""\u0000""#, r#""\uD83D\uDE00""#, r#""\U0041""#] {
    println!("{s:?} => {}", r::<String>(s));
}`);
// B. options and collections
p("opt_nesting", "", R`println!("{} {} {} {}", w(&Some(1)), w(&None::<i32>), w(&Some(None::<i32>)), w(&Some(Some(1))));
println!("{}", w(&vec![Some(1), None]));
println!("{} {} {}", r::<Option<i32>>("null"), r::<Option<i32>>("5"), r::<Option<Option<i32>>>("null"));
println!("{} {}", r::<Option<String>>("\"x\""), r::<Option<i32>>("\"x\""));`);
p("seq_and_tuples", "", R`println!("{} {} {} {}", w(&vec![1, 2, 3]), w(&[[1, 2], [3, 4]]), w(&Vec::<i32>::new()), w(&(1, "a", true)));
println!("{} {} {}", w(&(1,)), w(&((1, 2), [3])), w(&[0u8; 0]));
println!("{} {} {}", r::<(i32, String)>("[1,\"a\"]"), r::<(i32, i32)>("[1]"), r::<(i32, i32)>("[1,2,3]"));
println!("{} {} {}", r::<[i32; 2]>("[1,2]"), r::<[i32; 2]>("[1,2,3]"), r::<[i32; 2]>("[1]"));
println!("{} {} {}", r::<Vec<i32>>("[1, 2 ,3 ]"), r::<Vec<i32>>("{}"), r::<Vec<i32>>("[1,\"a\"]"));`);
p("map_keys_write", R`#[derive(Serialize, Deserialize, Debug, PartialEq, Eq, PartialOrd, Ord)] enum K { A, B }`, R`println!("{}", w(&BTreeMap::from([("b", 1), ("a", 2)])));
println!("{}", w(&BTreeMap::from([(2, "x"), (-1, "y")])));
println!("{}", w(&BTreeMap::from([(true, 1), (false, 0)])));
println!("{}", w(&BTreeMap::from([('z', 1), ('a', 0)])));
println!("{}", w(&BTreeMap::from([(K::B, 1), (K::A, 0)])));
println!("{}", w(&BTreeMap::from([(1u64, 1)])));
println!("{}", w(&BTreeMap::from([((1, 2), 1)])));
println!("{}", w(&BTreeMap::from([(Some(1), 1)])));
println!("{}", w(&BTreeMap::from([(vec![1], 1)])));
println!("{}", w(&BTreeMap::from([(1.5f64.to_bits(), 1)])));`);
p("map_keys_read", R`#[derive(Serialize, Deserialize, Debug, PartialEq, Eq, PartialOrd, Ord)] enum K { A, B }`, R`println!("{}", r::<BTreeMap<String, i32>>(r#"{"b":1,"a":2}"#));
println!("{}", r::<BTreeMap<i32, String>>(r#"{"2":"x","-1":"y"}"#));
println!("{}", r::<BTreeMap<i32, String>>(r#"{"x":"x"}"#));
println!("{}", r::<BTreeMap<u8, i32>>(r#"{"256":1}"#));
println!("{}", r::<BTreeMap<bool, i32>>(r#"{"true":1}"#));
println!("{}", r::<BTreeMap<char, i32>>(r#"{"z":1}"#));
println!("{}", r::<BTreeMap<K, i32>>(r#"{"B":1,"A":0}"#));
println!("{}", r::<BTreeMap<K, i32>>(r#"{"C":1}"#));
println!("{}", r::<BTreeMap<String, i32>>(r#"{"a":1,"a":2}"#));
println!("{}", r::<BTreeMap<String, i32>>(r#"{1:2}"#));
println!("{}", r::<BTreeMap<i32, i32>>(r#"{" 1":2}"#));
`);
p("collections_std", "", R`println!("{} {}", w(&BTreeSet::from([3, 1, 2])), w(&VecDeque::from([1, 2])));
println!("{} {} {}", w(&Box::new(5)), w(&Box::new("rc")), w(&Cow::Borrowed("cow")));
println!("{} {}", w(&std::time::Duration::new(3, 5)), w(&(1..4)));
println!("{} {}", w(&Ok::<i32, String>(1)), w(&Err::<i32, String>("e".into())));
println!("{} {}", w(&std::cell::Cell::new(1)), w(&std::cell::RefCell::new(vec![1])));
println!("{} {}", w(&std::num::Wrapping(7u8)), w(&std::cmp::Reverse(3)));
println!("{} {}", w(&std::path::PathBuf::from("a/b")), w(&std::num::NonZeroU32::new(9)));
println!("{} {}", r::<BTreeSet<i32>>("[3,1,3]"), r::<VecDeque<i32>>("[1,2]"));
println!("{} {}", r::<Result<i32, String>>(r#"{"Ok":1}"#), r::<Result<i32, String>>(r#"{"Err":"e"}"#));
println!("{} {}", r::<std::time::Duration>(r#"{"secs":3,"nanos":5}"#), r::<std::ops::Range<i32>>(r#"{"start":1,"end":4}"#));
println!("{} {}", r::<Box<i32>>("5"), r::<std::num::NonZeroU32>("0"));`);
// C. structs
p("struct_kinds", R`#[derive(Serialize, Deserialize, Debug)] struct Named { a: i32, b: String }
#[derive(Serialize, Deserialize, Debug)] struct Unit;
#[derive(Serialize, Deserialize, Debug)] struct New(i32);
#[derive(Serialize, Deserialize, Debug)] struct Tup(i32, String);
#[derive(Serialize, Deserialize, Debug)] struct Empty {}
#[derive(Serialize, Deserialize, Debug)] struct Nest { n: Named, t: Tup, u: Unit, e: Empty, w: New }`, R`println!("{} {} {} {} {}", w(&Named { a: 1, b: "x".into() }), w(&Unit), w(&New(5)), w(&Tup(1, "t".into())), w(&Empty {}));
println!("{}", w(&Nest { n: Named { a: 1, b: "x".into() }, t: Tup(2, "t".into()), u: Unit, e: Empty {}, w: New(3) }));
println!("{}", r::<Named>(r#"{"b":"x","a":1}"#));
println!("{}", r::<Named>(r#"[1,"x"]"#));
println!("{}", r::<Named>(r#"[1]"#));
println!("{}", r::<Named>(r#"{"a":1}"#));
println!("{}", r::<Named>(r#"{"a":1,"b":"x","c":[1,{"d":null}]}"#));
println!("{}", r::<Named>(r#"{"a":"1","b":"x"}"#));
println!("{}", r::<Named>(r#"{"a":1,"a":2,"b":"x"}"#));
println!("{} {} {}", r::<Unit>("null"), r::<Unit>("[]"), r::<Unit>("{}"));
println!("{} {} {}", r::<New>("5"), r::<New>("[5]"), r::<Tup>(r#"[1,"t"]"#));
println!("{} {} {}", r::<Tup>(r#"[1]"#), r::<Tup>(r#"[1,"t",3]"#), r::<Empty>("{}"));
println!("{} {}", r::<Empty>("[]"), r::<Empty>(r#"{"x":1}"#));`);
for (const [i, rule] of ["lowercase", "UPPERCASE", "PascalCase", "camelCase", "snake_case", "SCREAMING_SNAKE_CASE", "kebab-case", "SCREAMING-KEBAB-CASE"].entries()) {
  p(`rename_all_${i}_fields`, `#[derive(Serialize, Deserialize, Debug, Default)] #[serde(rename_all = "${rule}")] struct S { my_field_name: i32, x2_y: i32, http_url2: i32, a: i32, _private: i32, trailing_: i32, double__under: i32, r#type: i32, field1: i32, a_b_c: i32 }`, R`let text = w(&S::default());
println!("{text}");
println!("{}", r::<S>(&text));`);
  p(`rename_all_${i}_variants`, `#[derive(Serialize, Deserialize, Debug)] #[serde(rename_all = "${rule}")] enum E { MyVariantName, HTTPServer, X2Y, A, Abc1, ABc, Struct { my_field: i32 } }`, R`let all = vec![E::MyVariantName, E::HTTPServer, E::X2Y, E::A, E::Abc1, E::ABc, E::Struct { my_field: 1 }];
let text = w(&all);
println!("{text}");
println!("{}", r::<Vec<E>>(&text));`);
}
p("rename_fields", R`#[derive(Serialize, Deserialize, Debug)] struct S { #[serde(rename = "type")] kind: i32, #[serde(rename(serialize = "out", deserialize = "in"))] io: i32, #[serde(rename = "with space")] sp: i32, #[serde(rename = "é\"q")] odd: i32 }`, R`println!("{}", w(&S { kind: 1, io: 2, sp: 3, odd: 4 }));
println!("{}", r::<S>(r#"{"type":1,"in":2,"with space":3,"é\"q":4}"#));
println!("{}", r::<S>(r#"{"type":1,"out":2,"with space":3,"é\"q":4}"#));`);
p("skip_fields", R`fn is_zero(n: &i32) -> bool { *n == 0 }
#[derive(Serialize, Deserialize, Debug, Default)] struct S { a: i32, #[serde(skip)] b: i32, #[serde(skip_serializing)] c: i32, #[serde(skip_deserializing)] d: i32, #[serde(skip_serializing_if = "Option::is_none")] e: Option<i32>, #[serde(skip_serializing_if = "Vec::is_empty")] f: Vec<i32>, #[serde(skip_serializing_if = "is_zero")] g: i32, #[serde(default, skip_serializing_if = "String::is_empty")] h: String }`, R`println!("{}", w(&S { a: 1, b: 2, c: 3, d: 4, e: None, f: vec![], g: 0, h: String::new() }));
println!("{}", w(&S { a: 1, b: 2, c: 3, d: 4, e: Some(5), f: vec![6], g: 7, h: "h".into() }));
println!("{}", r::<S>(r#"{"a":1,"b":2,"c":3,"d":4,"e":5,"f":[6],"g":7,"h":"h"}"#));
println!("{}", r::<S>(r#"{"a":1,"c":3,"f":[],"g":0}"#));
println!("{}", r::<S>(r#"{"a":1,"f":[],"g":0}"#));`);
p("default_fields", R`fn seven() -> i32 { 7 }
#[derive(Serialize, Deserialize, Debug)] struct S { a: i32, #[serde(default)] b: i32, #[serde(default = "seven")] c: i32, d: Option<i32>, #[serde(default)] e: Vec<String>, #[serde(default)] f: Option<i32> }
#[derive(Serialize, Deserialize, Debug)] #[serde(default)] struct C { a: i32, b: String, c: bool }
impl Default for C { fn default() -> C { C { a: 9, b: "dflt".into(), c: true } } }`, R`println!("{}", r::<S>(r#"{"a":1}"#));
println!("{}", r::<S>(r#"{}"#));
println!("{}", r::<S>(r#"{"a":1,"b":null}"#));
println!("{}", r::<S>(r#"{"a":1,"d":null,"f":null}"#));
println!("{}", r::<C>(r#"{}"#));
println!("{}", r::<C>(r#"{"b":"x"}"#));`);
p("alias_fields", R`#[derive(Serialize, Deserialize, Debug)] struct S { #[serde(alias = "A", alias = "alpha")] a: i32, #[serde(rename = "bee", alias = "b")] b: i32 }`, R`println!("{}", w(&S { a: 1, b: 2 }));
println!("{}", r::<S>(r#"{"A":1,"b":2}"#));
println!("{}", r::<S>(r#"{"alpha":1,"bee":2}"#));
println!("{}", r::<S>(r#"{"a":1,"A":1,"bee":2}"#));
println!("{}", r::<S>(r#"{"a":1,"bee":2,"b":3}"#));`);
p("deny_unknown", R`#[derive(Serialize, Deserialize, Debug)] #[serde(deny_unknown_fields)] struct S { a: i32, b: i32 }
#[derive(Serialize, Deserialize, Debug)] #[serde(deny_unknown_fields)] struct One { only: i32 }
#[derive(Serialize, Deserialize, Debug)] #[serde(deny_unknown_fields)] struct Zero {}`, R`println!("{}", r::<S>(r#"{"a":1,"b":2,"c":3}"#));
println!("{}", r::<S>(r#"{"c":3,"a":1,"b":2}"#));
println!("{}", r::<One>(r#"{"only":1,"x":2}"#));
println!("{}", r::<Zero>(r#"{"x":2}"#));
println!("{}", r::<S>(r#"{"a":1,"b":2}"#));`);
p("flatten_struct", R`#[derive(Serialize, Deserialize, Debug)] struct Inner { x: i32, y: i32 }
#[derive(Serialize, Deserialize, Debug)] struct Outer { id: i32, #[serde(flatten)] inner: Inner, tail: bool }
#[derive(Serialize, Deserialize, Debug)] struct Two { #[serde(flatten)] a: Inner, #[serde(flatten)] b: More }
#[derive(Serialize, Deserialize, Debug)] struct More { z: i32 }
#[derive(Serialize, Deserialize, Debug)] struct Opt { id: i32, #[serde(flatten)] inner: Option<Inner> }`, R`println!("{}", w(&Outer { id: 1, inner: Inner { x: 2, y: 3 }, tail: true }));
println!("{}", r::<Outer>(r#"{"y":3,"tail":false,"x":2,"id":1}"#));
println!("{}", r::<Outer>(r#"{"id":1,"x":2,"tail":true}"#));
println!("{}", r::<Outer>(r#"{"id":1,"x":2,"y":3,"tail":true,"extra":9}"#));
println!("{}", w(&Two { a: Inner { x: 1, y: 2 }, b: More { z: 3 } }));
println!("{}", r::<Two>(r#"{"z":3,"y":2,"x":1}"#));
println!("{} {}", w(&Opt { id: 1, inner: None }), w(&Opt { id: 1, inner: Some(Inner { x: 1, y: 2 }) }));
println!("{} {}", r::<Opt>(r#"{"id":1}"#), r::<Opt>(r#"{"id":1,"x":1,"y":2}"#));
println!("{}", r::<Opt>(r#"{"id":1,"x":1}"#));`);
p("flatten_map", R`#[derive(Serialize, Deserialize, Debug)] struct S { id: i32, #[serde(flatten)] extra: BTreeMap<String, Value> }
#[derive(Serialize, Deserialize, Debug)] struct T { id: i32, #[serde(flatten)] extra: BTreeMap<String, i32> }`, R`let s = S { id: 1, extra: BTreeMap::from([("k".to_string(), json!([1, "a"])), ("a".to_string(), json!(null))]) };
println!("{}", w(&s));
println!("{}", r::<S>(r#"{"z":{"deep":true},"id":1,"a":1.5}"#));
println!("{}", r::<T>(r#"{"id":1,"a":2,"b":3}"#));
println!("{}", r::<T>(r#"{"id":1,"a":"x"}"#));`);
p("flatten_enum", R`#[derive(Serialize, Deserialize, Debug)] enum Kind { A { v: i32 }, B(String) }
#[derive(Serialize, Deserialize, Debug)] struct S { id: i32, #[serde(flatten)] kind: Kind }
#[derive(Serialize, Deserialize, Debug)] #[serde(tag = "t")] enum Tagged { A { v: i32 }, B }
#[derive(Serialize, Deserialize, Debug)] struct U { id: i32, #[serde(flatten)] kind: Tagged }`, R`println!("{} {}", w(&S { id: 1, kind: Kind::A { v: 2 } }), w(&S { id: 1, kind: Kind::B("b".into()) }));
println!("{} {}", r::<S>(r#"{"id":1,"A":{"v":2}}"#), r::<S>(r#"{"B":"b","id":1}"#));
println!("{} {}", w(&U { id: 1, kind: Tagged::A { v: 2 } }), w(&U { id: 1, kind: Tagged::B }));
println!("{} {}", r::<U>(r#"{"v":2,"id":1,"t":"A"}"#), r::<U>(r#"{"id":1,"t":"B"}"#));
println!("{}", r::<U>(r#"{"id":1}"#));`);
p("transparent_from_into", R`#[derive(Serialize, Deserialize, Debug)] #[serde(transparent)] struct Id(u32);
#[derive(Serialize, Deserialize, Debug)] #[serde(transparent)] struct Wrap { inner: Vec<i32> }
#[derive(Serialize, Deserialize, Debug, Clone)] #[serde(from = "String", into = "String")] struct Upper(String);
impl From<String> for Upper { fn from(s: String) -> Upper { Upper(s.to_uppercase()) } }
impl From<Upper> for String { fn from(u: Upper) -> String { format!("<{}>", u.0) } }
#[derive(Serialize, Deserialize, Debug)] #[serde(try_from = "i32")] struct Even(i32);
impl TryFrom<i32> for Even { type Error = String; fn try_from(n: i32) -> Result<Even, String> { if n % 2 == 0 { Ok(Even(n)) } else { Err(format!("{n} is odd")) } } }`, R`println!("{} {}", w(&Id(7)), w(&Wrap { inner: vec![1] }));
println!("{} {}", r::<Id>("7"), r::<Wrap>("[1,2]"));
println!("{} {}", w(&Upper("abc".into())), r::<Upper>("\"abc\""));
println!("{} {} {}", r::<Even>("4"), r::<Even>("3"), r::<Vec<Even>>("[2,\n 5]"));`);
p("with_functions", R`mod as_text {
    use serde::{Deserialize, Deserializer, Serializer};
    pub fn serialize<S: Serializer>(n: &u64, s: S) -> Result<S::Ok, S::Error> { s.serialize_str(&n.to_string()) }
    pub fn deserialize<'de, D: Deserializer<'de>>(d: D) -> Result<u64, D::Error> {
        let text = String::deserialize(d)?;
        text.parse().map_err(serde::de::Error::custom)
    }
}
fn doubled<S: serde::Serializer>(n: &i32, s: S) -> Result<S::Ok, S::Error> { s.serialize_i32(n * 2) }
fn halved<'de, D: serde::Deserializer<'de>>(d: D) -> Result<i32, D::Error> { Ok(i32::deserialize(d)? / 2) }
#[derive(Serialize, Deserialize, Debug)] struct S { #[serde(with = "as_text")] big: u64, #[serde(serialize_with = "doubled", deserialize_with = "halved")] n: i32 }`, R`println!("{}", w(&S { big: u64::MAX, n: 4 }));
println!("{}", r::<S>(r#"{"big":"18446744073709551615","n":8}"#));
println!("{}", r::<S>(r#"{"big":"nope","n":8}"#));
println!("{}", r::<S>(r#"{"big":5,"n":8}"#));`);
p("borrowed_and_generic", R`#[derive(Serialize, Deserialize, Debug)] struct B<'a> { name: &'a str, #[serde(borrow)] note: Cow<'a, str>, n: i32 }
#[derive(Serialize, Deserialize, Debug)] struct W<T> { inner: T, list: Vec<T> }
#[derive(Serialize, Deserialize, Debug)] struct P<T> { v: i32, #[serde(skip)] m: std::marker::PhantomData<T> }`, R`let text = String::from(r#"{"name":"n","note":"plain","n":1}"#);
let b: B = serde_json::from_str(&text).unwrap();
println!("{b:?} {}", w(&b));
let esc = String::from(r#"{"name":"a\nb","note":"x","n":1}"#);
println!("{:?}", serde_json::from_str::<B>(&esc).map_err(|e| e.to_string()));
println!("{} {}", w(&W { inner: 1, list: vec![2] }), r::<W<String>>(r#"{"inner":"a","list":["b"]}"#));
println!("{} {}", w(&W { inner: Some(W { inner: 1, list: vec![] }), list: vec![] }), r::<P<String>>(r#"{"v":1}"#));`);
p("type_errors", R`#[derive(Serialize, Deserialize, Debug)] struct P { x: i32, y: i32 }
#[derive(Serialize, Deserialize, Debug)] struct Q { p: P, name: String, tags: Vec<String> }`, R`for s in [r#"{"x":1}"#, r#"{"x":1,"y":"2"}"#, r#"{"x":1,"y":2.5}"#, r#"{"x":1,"y":null}"#, r#"[1,2]"#, r#"[1,2,3]"#, r#""p""#, "7", "null", "true", r#"{"x":1,"y":2"#, r#"{"x":1,,"y":2}"#, r#"{"x":99999999999,"y":2}"#] {
    println!("{s} => {}", r::<P>(s));
}
for s in ["{\n  \"p\": {\"x\": 1, \"y\": true},\n  \"name\": \"n\",\n  \"tags\": []\n}", "{\n  \"p\": {\"x\": 1, \"y\": 2},\n  \"name\": 5,\n  \"tags\": []\n}", "{\"p\":{\"x\":1,\"y\":2},\"name\":\"n\",\"tags\":[\"a\",1]}", "{\"p\":{\"x\":1,\"y\":2},\"name\":\"n\"}"] {
    println!("=> {}", r::<Q>(s));
}`);
// D. enums
p("enum_external", R`#[derive(Serialize, Deserialize, Debug)] enum E { Unit, New(i32), Tup(i32, String), Struct { a: i32, b: Option<i32> }, Nested(Box<E>), Empty {}, NoArgs() }`, R`let all = vec![E::Unit, E::New(1), E::Tup(2, "t".into()), E::Struct { a: 3, b: None }, E::Nested(Box::new(E::Unit)), E::Empty {}, E::NoArgs()];
let text = w(&all);
println!("{text}");
println!("{}", r::<Vec<E>>(&text));
for s in [r#""Unit""#, r#"{"Unit":null}"#, r#"{"Unit":1}"#, r#""New""#, r#"{"New":1,"Tup":[1,"a"]}"#, r#"{"Nope":1}"#, r#""Nope""#, r#"{"Tup":[1]}"#, r#"{"Struct":{"a":1}}"#, r#"{"Struct":[1,2]}"#, r#"{}"#, "1", "null", r#"{"Empty":{}}"#, r#"{"NoArgs":[]}"#] {
    println!("{s} => {}", r::<E>(s));
}`);
p("enum_internal", R`#[derive(Serialize, Deserialize, Debug)] struct Body { n: i32 }
#[derive(Serialize, Deserialize, Debug)] #[serde(tag = "type")] enum E { Unit, Struct { a: i32 }, New(Body), Map(BTreeMap<String, i32>), Empty {} }`, R`let all = vec![E::Unit, E::Struct { a: 1 }, E::New(Body { n: 2 }), E::Map(BTreeMap::from([("k".to_string(), 3)])), E::Empty {}];
let text = w(&all);
println!("{text}");
println!("{}", r::<Vec<E>>(&text));
for s in [r#"{"a":1,"type":"Struct"}"#, r#"{"type":"Unit","extra":1}"#, r#"{"a":1}"#, r#"{"type":"Nope"}"#, r#"{"type":1}"#, r#""Unit""#, r#"["Unit"]"#, r#"{"type":"Struct"}"#, r#"{"type":"New","n":2,"x":1}"#, r#"{"type":"Struct","a":1,"type":"Unit"}"#] {
    println!("{s} => {}", r::<E>(s));
}`);
p("enum_adjacent", R`#[derive(Serialize, Deserialize, Debug)] #[serde(tag = "t", content = "c")] enum E { Unit, New(i32), Tup(i32, String), Struct { a: i32 }, Opt(Option<i32>) }`, R`let all = vec![E::Unit, E::New(1), E::Tup(2, "t".into()), E::Struct { a: 3 }, E::Opt(None), E::Opt(Some(4))];
let text = w(&all);
println!("{text}");
println!("{}", r::<Vec<E>>(&text));
for s in [r#"{"c":1,"t":"New"}"#, r#"{"t":"Unit"}"#, r#"{"t":"Unit","c":null}"#, r#"{"t":"New"}"#, r#"{"c":1}"#, r#"{"t":"Nope","c":1}"#, r#"{"t":"New","c":"x"}"#, r#"["New",1]"#, r#"{"t":"Opt"}"#, r#"{"t":"New","c":1,"x":2}"#] {
    println!("{s} => {}", r::<E>(s));
}`);
p("enum_untagged", R`#[derive(Serialize, Deserialize, Debug)] #[serde(untagged)] enum E { Int(i32), Float(f64), Text(String), Pair(i32, i32), List(Vec<i32>), Point { x: i32, y: i32 }, Named { name: String }, Unit }`, R`let all = vec![E::Int(1), E::Float(1.5), E::Text("t".into()), E::Pair(1, 2), E::List(vec![1, 2, 3]), E::Point { x: 1, y: 2 }, E::Named { name: "n".into() }, E::Unit];
let text = w(&all);
println!("{text}");
println!("{}", r::<Vec<E>>(&text));
for s in ["1", "1.0", "3000000000", r#""x""#, "[1,2]", "[1]", "[]", r#"{"x":1,"y":2}"#, r#"{"x":1,"y":2,"z":3}"#, r#"{"name":"n","x":1}"#, "null", "true", r#"{"x":1}"#, r#"[1,"a"]"#] {
    println!("{s} => {}", r::<E>(s));
}`);
p("enum_attrs", R`#[derive(Serialize, Deserialize, Debug)] enum E { #[serde(rename = "first")] A, #[serde(alias = "bee", alias = "Bee")] B, #[serde(skip)] C, #[serde(other)] Other }
#[derive(Serialize, Deserialize, Debug)] #[serde(rename_all_fields = "camelCase")] enum F { One { my_field: i32 }, Two { other_one: i32 } }
#[derive(Serialize, Deserialize, Debug)] enum G { A, #[serde(skip_serializing)] B, #[serde(skip_deserializing)] C }`, R`println!("{} {} {} {}", w(&E::A), w(&E::B), w(&E::C), w(&E::Other));
for s in [r#""first""#, r#""A""#, r#""bee""#, r#""B""#, r#""C""#, r#""whatever""#, "1"] { println!("{s} => {}", r::<E>(s)); }
println!("{} {}", w(&F::One { my_field: 1 }), r::<F>(r#"{"Two":{"otherOne":2}}"#));
println!("{} {} {}", w(&G::A), w(&G::B), w(&G::C));
println!("{} {} {}", r::<G>(r#""A""#), r::<G>(r#""B""#), r::<G>(r#""C""#));`);
p("enum_c_like", R`#[derive(Serialize, Deserialize, Debug, Clone, Copy, PartialEq)] #[repr(u8)] enum Level { Low = 1, Mid = 5, High = 9 }
#[derive(Serialize, Deserialize, Debug)] struct S { level: Level, all: Vec<Level>, by: BTreeMap<String, Level>, opt: Option<Level> }`, R`let s = S { level: Level::Mid, all: vec![Level::Low, Level::High], by: BTreeMap::from([("k".to_string(), Level::Low)]), opt: None };
let text = w(&s);
println!("{text}");
println!("{}", r::<S>(&text));
println!("{} {} {}", r::<Level>("\"Mid\""), r::<Level>("5"), r::<Level>("\"mid\""));
println!("{}", Level::High as u8);`);
// E. serde_json API
p("pretty", R`#[derive(Serialize)] struct S { a: i32, b: Vec<i32>, c: BTreeMap<String, Option<bool>>, d: (), e: Vec<Vec<i32>>, f: Empty, g: String }
#[derive(Serialize)] struct Empty {}`, R`let s = S { a: 1, b: vec![], c: BTreeMap::from([("k".to_string(), Some(true)), ("n".to_string(), None)]), d: (), e: vec![vec![], vec![1, 2]], f: Empty {}, g: "é\n".into() };
println!("{}", serde_json::to_string_pretty(&s).unwrap());
println!("{}", serde_json::to_string_pretty(&json!({"a": [], "b": {}, "c": [{}], "d": [[]], "e": null})).unwrap());
println!("{}", serde_json::to_string_pretty(&1).unwrap());
println!("{}", serde_json::to_string_pretty(&Vec::<i32>::new()).unwrap());
println!("{:#}", json!({"k": [1, {"z": 2}]}));`);
p("bytes_and_writers", R`#[derive(Serialize, Deserialize, Debug)] struct P { x: i32 }`, R`let bytes = serde_json::to_vec(&P { x: 1 }).unwrap();
println!("{bytes:?}");
let pretty = serde_json::to_vec_pretty(&P { x: 1 }).unwrap();
println!("{}", String::from_utf8(pretty).unwrap());
let mut out: Vec<u8> = Vec::new();
serde_json::to_writer(&mut out, &vec![1, 2]).unwrap();
println!("{}", String::from_utf8(out).unwrap());
println!("{:?}", serde_json::from_slice::<P>(b"{\"x\":5}").map_err(|e| e.to_string()));
println!("{:?}", serde_json::from_slice::<P>(&[0xff, 0xfe]).map_err(|e| e.to_string()));
println!("{:?}", serde_json::from_slice::<String>(b"\"\xff\"").map_err(|e| e.to_string()));
println!("{:?}", serde_json::from_reader::<_, P>(&b"{\"x\":6}"[..]).map_err(|e| e.to_string()));
println!("{:?}", serde_json::from_reader::<_, P>(&b"{\"x\":"[..]).map_err(|e| format!("{e} {:?}", e.classify())));`);
p("value_macro", "", R`let name = "n"; let n = 3; let list = vec![1, 2]; let opt: Option<i32> = None;
let v = json!({"name": name, "n": n, "list": list, "opt": opt, "nested": {"a": [1, 2.5, "x", null, true], "b": {}}, "calc": n * 2, "neg": -1, "f": 1.0, "big": u64::MAX});
println!("{v}");
println!("{}", json!([1, [2, [3]], {"k": []}]));
println!("{} {} {} {}", json!(null), json!("s"), json!(1.5), json!(true));
println!("{}", json!({"b": 1, "a": 2, "c": {"z": 1, "y": 2}}));`);
p("value_access", "", R`let v: Value = serde_json::from_str(r#"{"a":[1,2,{"b":"x"}],"n":-5,"u":18446744073709551615,"f":1.5,"s":"str","t":true,"z":null,"o":{"k":1}}"#).unwrap();
println!("{} {} {} {}", v["a"][0], v["a"][2]["b"], v["missing"], v["a"][9]);
println!("{:?} {:?} {:?} {:?}", v["n"].as_i64(), v["n"].as_u64(), v["n"].as_f64(), v["u"].as_i64());
println!("{:?} {:?} {:?} {:?}", v["u"].as_u64(), v["f"].as_i64(), v["f"].as_f64(), v["s"].as_str());
println!("{:?} {:?} {:?}", v["t"].as_bool(), v["z"].as_null(), v["a"].as_array().map(|a| a.len()));
println!("{} {} {} {} {} {} {}", v["n"].is_number(), v["n"].is_i64(), v["u"].is_u64(), v["f"].is_f64(), v["s"].is_string(), v["z"].is_null(), v["o"].is_object());
println!("{:?} {:?} {:?}", v.get("a").and_then(|a| a.get(1)), v.get("nope"), v["a"].get(5));
println!("{:?} {:?} {:?} {:?}", v.pointer("/a/2/b"), v.pointer("/o/k"), v.pointer("/a/x"), v.pointer(""));
println!("{} {} {} {}", v["n"] == -5, v["s"] == "str", v["t"] == true, v["f"] == 1.5);
println!("{:?}", v["o"].as_object().map(|o| o.keys().cloned().collect::<Vec<_>>()));
println!("{:?} {:?}", v, v["a"]);`);
p("value_mutate", "", R`let mut v = json!({"a": 1});
v["b"] = json!([1, 2]);
v["a"] = json!("changed");
v["c"]["d"] = json!(true);
v["b"][0] = json!(null);
println!("{v}");
let taken = v["b"].take();
println!("{taken} {v}");
if let Some(map) = v.as_object_mut() { map.insert("z".into(), json!(1)); map.remove("a"); }
println!("{v} {}", v.as_object().map(|m| m.len()).unwrap_or(0));
if let Value::Array(items) = &mut json!([3, 1]) { items.push(json!(2)); println!("{}", Value::Array(items.clone())); }
let mut arr = json!([1]);
arr.as_array_mut().unwrap().push(json!({"k": "v"}));
println!("{arr} {}", arr[1]["k"]);`);
p("value_convert", R`#[derive(Serialize, Deserialize, Debug)] struct P { x: i32, tags: Vec<String>, opt: Option<f64> }`, R`let v = serde_json::to_value(P { x: 1, tags: vec!["a".into()], opt: Some(2.5) }).unwrap();
println!("{v}");
println!("{:?}", serde_json::from_value::<P>(v.clone()).map_err(|e| e.to_string()));
println!("{:?}", serde_json::from_value::<P>(json!({"x": "no", "tags": []})).map_err(|e| e.to_string()));
println!("{:?}", serde_json::from_value::<P>(json!({"tags": []})).map_err(|e| e.to_string()));
println!("{:?}", serde_json::from_value::<Vec<i32>>(json!([1, 2.5])).map_err(|e| e.to_string()));
println!("{:?}", serde_json::from_value::<BTreeMap<String, Value>>(json!({"k": [1]})).map_err(|e| e.to_string()));
println!("{} {}", serde_json::to_value(u64::MAX).unwrap(), serde_json::to_value(f64::NAN).unwrap());
println!("{} {}", serde_json::to_value(BTreeMap::from([(1, 2)])).unwrap(), serde_json::to_value((1, "a")).unwrap());
println!("{:?}", serde_json::to_value(BTreeMap::from([((1, 2), 3)])).map_err(|e| e.to_string()));`);
p("value_numbers", "", R`let v: Value = serde_json::from_str("[1, 1.0, -1, 1e2, 18446744073709551615, -9223372036854775808, 0.1, 1e-7, 1e21, 5e-324, 1.7976931348623157e308, 100000000000000000000, 0, -0, -0.0]").unwrap();
println!("{v}");
for x in v.as_array().unwrap() { print!("{}{}{} ", x.is_i64() as u8, x.is_u64() as u8, x.is_f64() as u8); }
println!();
println!("{} {} {}", json!(1) == json!(1.0), json!(1u64) == json!(1i64), json!(0.0) == json!(-0.0));
println!("{:?} {:?}", serde_json::Number::from_f64(1.0), serde_json::Number::from(5u8));`);
p("value_keys_order", "", R`let v: Value = serde_json::from_str(r#"{"b":1,"a":2,"c":{"z":1,"y":2},"10":0,"9":0,"B":0,"é":0,"a":3}"#).unwrap();
println!("{v}");
println!("{:?}", v.as_object().unwrap().iter().map(|(k, v)| format!("{k}={v}")).collect::<Vec<_>>());
let m: HashMap<String, i32> = serde_json::from_str(r#"{"only":1}"#).unwrap();
println!("{}", w(&m));`);
p("errors_syntax", "", R`for s in ["", " ", "{", "[1,", "[1 2]", "{\"a\"}", "{\"a\":}", "{\"a\":1,}", "[1,]", "nul", "tru", "falsy", "\"abc", "1 2", "{\"a\":1}x", "{1:2}", "{'a':1}", "[1,,2]", "]", "}", ":", ",", "[\"a\" \"b\"]", "{\"a\" 1}", "{\"a\":1 \"b\":2}", "\u{feff}1", "1\u{a0}", "/* c */ 1", "// c\n1", "[1] // c", "nulll", "truefalse", "-", "\"\\", "[\"\\u12\"]", "{\"a\":1}}", "[[]", "\t\r\n 1 \t\r\n"] {
    println!("{s:?} => {}", r::<Value>(s));
}`);
p("errors_lines", "", R`for s in ["{\n  \"a\": [1,\n 2,, 3]\n}", "[\n\n\n  tru]", "{\"a\":\n\"b\nc\"}", "\n\n", "[1,\r\n2,\r\n]", "{\"k\": \"v\"\n \"k2\": 1}", "  [1,\n   {\"a\": nul}]"] {
    println!("{s:?} => {}", r::<Value>(s));
}`);
p("errors_depth", "", R`let deep = "[".repeat(127) + &"]".repeat(127);
println!("{}", r::<Value>(&deep).len());
let deeper = "[".repeat(128) + &"]".repeat(128);
println!("{}", r::<Value>(&deeper));
let objs = "{\"a\":".repeat(128) + "1" + &"}".repeat(128);
println!("{}", r::<Value>(&objs));
let unclosed = "[".repeat(50);
println!("{}", r::<Value>(&unclosed));`);
p("error_api", R`#[derive(Deserialize, Debug)] struct P { x: i32 }`, R`let e = serde_json::from_str::<P>("{\"x\": \"s\"}").unwrap_err();
println!("{e} | {e:?} | {} {} | {} {} {} {}", e.line(), e.column(), e.is_syntax(), e.is_data(), e.is_eof(), e.is_io());
let e = serde_json::from_str::<P>("{\"x\": ").unwrap_err();
println!("{e} | {e:?} | {:?} {}", e.classify(), e.is_eof());
let e = serde_json::from_str::<P>("{x}").unwrap_err();
println!("{e} | {:?}", e.classify());
let e: Box<dyn std::error::Error> = Box::new(serde_json::from_str::<P>("?").unwrap_err());
println!("{e} | {}", e.source().is_none());
let io: std::io::Error = serde_json::from_str::<P>("{").unwrap_err().into();
println!("{:?} {io}", io.kind());`);
p("custom_errors", R`#[derive(Debug)] struct Port(u16);
impl<'de> Deserialize<'de> for Port {
    fn deserialize<D: serde::Deserializer<'de>>(d: D) -> Result<Port, D::Error> {
        let n = u32::deserialize(d)?;
        if n > 65535 { return Err(serde::de::Error::custom(format!("port {n} is too large"))); }
        if n == 0 { return Err(serde::de::Error::invalid_value(serde::de::Unexpected::Unsigned(0), &"a non-zero port")); }
        Ok(Port(n as u16))
    }
}
#[derive(Deserialize, Debug)] struct Cfg { port: Port, host: String }`, R`for s in [r#"{"port":80,"host":"h"}"#, r#"{"port":70000,"host":"h"}"#, "{\n \"port\": 0,\n \"host\": \"h\"}", r#"{"port":"x","host":"h"}"#, r#"{"host":"h"}"#] {
    println!("{s:?} => {}", r::<Cfg>(s));
}`);
p("roundtrip_everything", R`#[derive(Serialize, Deserialize, Debug, PartialEq)] #[serde(rename_all = "camelCase")] struct Doc { doc_id: u64, #[serde(default)] tags: Vec<String>, meta: BTreeMap<String, Value>, #[serde(flatten)] body: Body, owner: Option<Owner>, scores: Vec<(String, f64)>, #[serde(rename = "type")] kind: Kind }
#[derive(Serialize, Deserialize, Debug, PartialEq)] struct Body { text: String, lines: u32 }
#[derive(Serialize, Deserialize, Debug, PartialEq)] struct Owner { name: String, #[serde(skip_serializing_if = "Option::is_none")] email: Option<String> }
#[derive(Serialize, Deserialize, Debug, PartialEq)] #[serde(tag = "k", content = "v", rename_all = "snake_case")] enum Kind { PlainText, Rich { level: u8 }, Linked(String) }`, R`let doc = Doc { doc_id: u64::MAX, tags: vec!["a\"b".into()], meta: BTreeMap::from([("m".to_string(), json!({"deep": [1, null]}))]), body: Body { text: "t\né".into(), lines: 2 }, owner: Some(Owner { name: "o".into(), email: None }), scores: vec![("s".into(), 0.1)], kind: Kind::Rich { level: 3 } };
let text = w(&doc);
println!("{text}");
let back: Doc = serde_json::from_str(&text).unwrap();
println!("{}", back == doc);
println!("{}", serde_json::to_string_pretty(&back).unwrap());
let via: Doc = serde_json::from_value(serde_json::to_value(&doc).unwrap()).unwrap();
println!("{}", via == doc);`);
p("stream_and_raw", "", R`let stream = serde_json::Deserializer::from_str("1 [2] {\"a\":3} \"x\" nul").into_iter::<Value>();
for item in stream { match item { Ok(v) => print!("{v} / "), Err(e) => print!("err {e} / ") } }
println!();
let mut de = serde_json::Deserializer::from_str("[1,2] trailing");
let v = Vec::<i32>::deserialize(&mut de);
println!("{v:?} {:?}", de.end().map_err(|e| e.to_string()));`);
p("hashmap_single_and_sets", "", R`let m: HashMap<String, Vec<Option<i32>>> = HashMap::from([("k".to_string(), vec![Some(1), None])]);
println!("{}", w(&m));
let s: HashSet<String> = HashSet::from(["only".to_string()]);
println!("{}", w(&s));
println!("{}", r::<HashMap<i64, bool>>(r#"{"-3":true}"#));
let nested: BTreeMap<String, BTreeMap<i32, Vec<(bool, char)>>> = BTreeMap::from([("o".to_string(), BTreeMap::from([(1, vec![(true, 'c')])]))]);
let text = w(&nested);
println!("{text} {}", r::<BTreeMap<String, BTreeMap<i32, Vec<(bool, char)>>>>(&text));`);
p("skip_unknown_values", R`#[derive(Deserialize, Debug)] struct S { keep: i32 }`, R`for s in [r#"{"skip":{"a":[1,{"b":"\"}]"}],"c":null},"keep":1}"#, r#"{"skip":"\ud83d\ude00\\","keep":2}"#, r#"{"skip":[[[[1e5,-0.0,true]]]],"keep":3}"#, r#"{"skip":{"a":},"keep":4}"#, r#"{"skip":"\ud800","keep":5}"#, r#"{"skip":01,"keep":6}"#, r#"{"keep":7,"skip":[1,}"#] {
    println!("{s} => {}", r::<S>(s));
}`);
mkdirSync("p", { recursive: true });
for (const [name, text] of Object.entries(P)) writeFileSync(`p/${name}.rs`, text);
console.log(Object.keys(P).length);
