// serde's protocol as a hand-written impl drives it, against native
// serde_json, the oracle, built of the same Rust and dependency versions
// (#1532's S1: #1544 to #1549, split from serde.test.ts by concept, #1334).
import { beforeAll, expect, test } from "bun:test";
import { readFileSync, writeFileSync } from "node:fs";
import { join } from "node:path";
import { compiled, native } from "./serde-support";
import { buildCompiler, buildSerde, fixture, run } from "./support";

beforeAll(() => {
  buildCompiler();
  buildSerde("rlib");
}, 600_000);

// A protocol call's `Result` is what the program does with it (#1545): one
// only `?` gives on, or that's returned, stays a throw, as it reads now;
// one the program looks at, a `match`'s, `is_err()`'s, `map_err`'s or a
// `let _ =`'s, is a value, `Ok` or the backend's `Err`, and what was
// written before the error stays written, as serde_json's writer keeps it,
// so a writer that writes on after it writes as Rust's does.
test("serde: a protocol call's Err is the program's to look at, as serde_json gives it", async () => {
  const dir = fixture("serde-protocol-result-inspected");
  writeFileSync(join(dir, "cases.rs"), String.raw`
use serde::ser::{Error, SerializeMap, SerializeSeq, SerializeStruct};
use serde::{Serialize, Serializer};

pub struct Bad;

impl Serialize for Bad {
    fn serialize<S: Serializer>(&self, _s: S) -> Result<S::Ok, S::Error> {
        Err(S::Error::custom("inner"))
    }
}

/// Writes part of itself, then fails.
pub struct Half;

impl Serialize for Half {
    fn serialize<S: Serializer>(&self, s: S) -> Result<S::Ok, S::Error> {
        let mut seq = s.serialize_seq(None)?;
        seq.serialize_element(&1u8)?;
        seq.serialize_element(&Bad)?;
        seq.end()
    }
}

/// Catches its element's error, and gives its own.
pub struct Caught;

impl Serialize for Caught {
    fn serialize<S: Serializer>(&self, s: S) -> Result<S::Ok, S::Error> {
        let mut seq = s.serialize_seq(Some(1))?;
        match seq.serialize_element(&Bad) {
            Ok(()) => seq.end(),
            Err(_) => Err(S::Error::custom("caught")),
        }
    }
}

/// Asks whether each element failed, and writes on.
pub struct WritesOn;

impl Serialize for WritesOn {
    fn serialize<S: Serializer>(&self, s: S) -> Result<S::Ok, S::Error> {
        let mut seq = s.serialize_seq(None)?;
        let first = seq.serialize_element(&Bad).is_err();
        let second = seq.serialize_element(&Half).is_err();
        seq.serialize_element(&(first, second))?;
        seq.end()
    }
}

/// Gives another error for its field's.
pub struct Mapped;

impl Serialize for Mapped {
    fn serialize<S: Serializer>(&self, s: S) -> Result<S::Ok, S::Error> {
        let mut st = s.serialize_struct("Mapped", 1)?;
        st.serialize_field("a", &Bad)
            .map_err(|_| S::Error::custom("field a"))?;
        st.end()
    }
}

/// Drops its entry's error, and ends.
pub struct Dropped;

impl Serialize for Dropped {
    fn serialize<S: Serializer>(&self, s: S) -> Result<S::Ok, S::Error> {
        let mut map = s.serialize_map(None)?;
        let _ = map.serialize_entry("bad", &Bad);
        map.serialize_entry("good", &2u8)?;
        map.end()
    }
}

/// Its whole value's Result asked, of a nested value.
pub struct Asked<T>(pub T);

impl<T: Serialize> Serialize for Asked<T> {
    fn serialize<S: Serializer>(&self, s: S) -> Result<S::Ok, S::Error> {
        let written = self.0.serialize(s);
        match written {
            Ok(ok) => Ok(ok),
            Err(_) => Err(S::Error::custom("asked")),
        }
    }
}

/// Panics as it's written: a panic, not an error, under an asked Result.
pub struct Panics;

impl Serialize for Panics {
    fn serialize<S: Serializer>(&self, _s: S) -> Result<S::Ok, S::Error> {
        panic!("panics")
    }
}

/// An error that says where it was made, from another by ?'s From.
pub struct At<E>(pub usize, pub E);

impl<E> From<E> for At<E> {
    fn from(e: E) -> Self {
        At(7, e)
    }
}

fn put<S: SerializeSeq>(seq: &mut S, value: &Bad) -> Result<(), At<S::Error>> {
    seq.serialize_element(value)?;
    Ok(())
}

/// Its element's error given on by ?, as another error.
pub struct Converted;

impl Serialize for Converted {
    fn serialize<S: Serializer>(&self, s: S) -> Result<S::Ok, S::Error> {
        let mut seq = s.serialize_seq(None)?;
        match put(&mut seq, &Bad) {
            Ok(()) => seq.end(),
            Err(At(at, _)) => Err(S::Error::custom(if at == 7 { "at 7" } else { "elsewhere" })),
        }
    }
}

fn both<T: Serialize>(value: &T) -> String {
    format!("{:?} {:?}", serde_json::to_string(value), serde_json::to_value(value))
}

pub fn report() -> String {
    let panicked = std::panic::catch_unwind(|| serde_json::to_string(&Asked(Panics)));
    [
        both(&Half),
        both(&Caught),
        both(&WritesOn),
        both(&Mapped),
        both(&Dropped),
        both(&Asked(Bad)),
        both(&Asked(3u8)),
        both(&Converted),
        format!("panicked {}", panicked.is_err()),
    ]
    .join("\n")
}
`);
  const expected = JSON.parse(run([native(dir)]));
  expect((await compiled(dir)).report()).toBe(expected);
  // Given on by `?` alone, its calls are the writer's steps as before, no
  // `$jsonTry` round any: #1532's cost gate on what reads as it did.
  const js = readFileSync(join(dir, "cases.js"), "utf8");
  expect(js).toContain("function halfSerialize_serialize(half, s) {\n  s.beginArray();\n  const seq = s;\n  seq.element();\n  seq.int(1);\n");
});

// A hand-written visitor's `expecting` is asked only where an error's
// message is made, each time one is, as serde formats a `&dyn Expected`
// (#1544): a read that succeeds never asks it, so its side effects, a
// count here, and its panic don't happen; its text is made as it's written
// into the message. Of `deserialize_bool`, `_any` and `_option`, from text
// and from a `Value`, a field's too.
test("serde: a visitor's expecting is asked only where an error says it", async () => {
  const dir = fixture("serde-protocol-expecting-lazy");
  writeFileSync(join(dir, "cases.rs"), String.raw`
use serde::de::{Error, Visitor};
use serde::{Deserialize, Deserializer};
use std::fmt;
use std::sync::atomic::{AtomicUsize, Ordering};

static EXPECTING: AtomicUsize = AtomicUsize::new(0);

fn asked() -> usize {
    EXPECTING.swap(0, Ordering::SeqCst)
}

#[derive(Debug)]
pub struct Flag(bool);

struct FlagVisitor;

impl<'de> Visitor<'de> for FlagVisitor {
    type Value = Flag;

    fn expecting(&self, f: &mut fmt::Formatter) -> fmt::Result {
        EXPECTING.fetch_add(1, Ordering::SeqCst);
        f.write_str("a flag")
    }

    fn visit_bool<E: Error>(self, b: bool) -> Result<Flag, E> {
        Ok(Flag(b))
    }
}

impl<'de> Deserialize<'de> for Flag {
    fn deserialize<D: Deserializer<'de>>(d: D) -> Result<Self, D::Error> {
        d.deserialize_bool(FlagVisitor)
    }
}

#[derive(Debug)]
pub struct Anything(String);

struct AnyVisitor;

impl<'de> Visitor<'de> for AnyVisitor {
    type Value = Anything;

    fn expecting(&self, f: &mut fmt::Formatter) -> fmt::Result {
        EXPECTING.fetch_add(1, Ordering::SeqCst);
        f.write_str("anything but a map")
    }

    fn visit_bool<E: Error>(self, b: bool) -> Result<Anything, E> {
        Ok(Anything(format!("bool {b}")))
    }

    fn visit_u64<E: Error>(self, n: u64) -> Result<Anything, E> {
        Ok(Anything(format!("u64 {n}")))
    }

    fn visit_str<E: Error>(self, s: &str) -> Result<Anything, E> {
        Ok(Anything(format!("str {s}")))
    }
}

impl<'de> Deserialize<'de> for Anything {
    fn deserialize<D: Deserializer<'de>>(d: D) -> Result<Self, D::Error> {
        d.deserialize_any(AnyVisitor)
    }
}

#[derive(Debug, Deserialize)]
pub struct Holder {
    flag: Flag,
    any: Anything,
}

#[derive(Debug)]
pub struct Maybe(Option<u8>);

struct MaybeVisitor;

impl<'de> Visitor<'de> for MaybeVisitor {
    type Value = Maybe;

    fn expecting(&self, f: &mut fmt::Formatter) -> fmt::Result {
        EXPECTING.fetch_add(1, Ordering::SeqCst);
        f.write_str("maybe a byte")
    }

    fn visit_none<E: Error>(self) -> Result<Maybe, E> {
        Ok(Maybe(None))
    }

    fn visit_some<D: Deserializer<'de>>(self, d: D) -> Result<Maybe, D::Error> {
        u8::deserialize(d).map(|b| Maybe(Some(b)))
    }
}

impl<'de> Deserialize<'de> for Maybe {
    fn deserialize<D: Deserializer<'de>>(d: D) -> Result<Self, D::Error> {
        d.deserialize_option(MaybeVisitor)
    }
}

#[derive(Debug)]
pub struct Strict(bool);

struct StrictVisitor;

impl<'de> Visitor<'de> for StrictVisitor {
    type Value = Strict;

    fn expecting(&self, _f: &mut fmt::Formatter) -> fmt::Result {
        panic!("expecting asked of a read that succeeds")
    }

    fn visit_bool<E: Error>(self, b: bool) -> Result<Strict, E> {
        Ok(Strict(b))
    }
}

impl<'de> Deserialize<'de> for Strict {
    fn deserialize<D: Deserializer<'de>>(d: D) -> Result<Self, D::Error> {
        d.deserialize_bool(StrictVisitor)
    }
}

fn read<T: for<'de> Deserialize<'de> + fmt::Debug>(text: &str) -> String {
    let from_text = serde_json::from_str::<T>(text);
    let after_text = asked();
    let value: serde_json::Value = serde_json::from_str(text).unwrap();
    let from_value = serde_json::from_value::<T>(value);
    let after_value = asked();
    format!("{from_text:?} asked {after_text}; {from_value:?} asked {after_value}")
}

// From text alone: a \`Value\`'s reader has no \`deserialize_option\` of a
// hand-written visitor yet (#1597).
fn read_text<T: for<'de> Deserialize<'de> + fmt::Debug>(text: &str) -> String {
    let from_text = serde_json::from_str::<T>(text);
    format!("{from_text:?} asked {}", asked())
}

pub fn report() -> String {
    [
        read::<Flag>("true"),
        read::<Flag>("1"),
        read::<Flag>("\"yes\""),
        read::<Anything>("7"),
        read::<Anything>("\"s\""),
        read::<Anything>("{}"),
        read::<Anything>("[1]"),
        read::<Holder>(r#"{"flag": false, "any": true}"#),
        read::<Holder>(r#"{"flag": 3, "any": true}"#),
        read::<Holder>(r#"{"flag": true, "any": {}}"#),
        read_text::<Maybe>("null"),
        read_text::<Maybe>("5"),
        read_text::<Maybe>("true"),
        read::<Strict>("false"),
    ]
    .join("\n")
}
`);
  const expected = JSON.parse(run([native(dir)]));
  expect((await compiled(dir)).report()).toBe(expected);
});

// A string is given to a hand-written visitor by the kind serde_json gives
// it (#1546): from the text, one with no escape borrowed,
// `visit_borrowed_str`, one with one by reference, `visit_str`; from a
// `Value` whole, `visit_string`; of an untagged enum's content by
// reference, `visit_str` but where it borrowed the text; of an internally
// tagged one's whole, `visit_string` but where it borrowed. Each falls back
// as serde's defaults do, to `visit_str`, and `visit_str`'s default is an
// error. By `deserialize_string`, `_any` and `_str`, and as fields.
test("serde: a string is given to a visitor as serde_json gives it, borrowed, by reference or whole", async () => {
  const dir = fixture("serde-protocol-string-kinds");
  writeFileSync(join(dir, "cases.rs"), String.raw`
use serde::de::{Error, Visitor};
use serde::{Deserialize, Deserializer};
use std::fmt;

/// Says which of its visitor's string methods read it.
#[derive(Debug)]
pub struct Said(String);

struct SaidVisitor;

impl<'de> Visitor<'de> for SaidVisitor {
    type Value = Said;

    fn expecting(&self, f: &mut fmt::Formatter) -> fmt::Result {
        f.write_str("a string")
    }

    fn visit_str<E: Error>(self, s: &str) -> Result<Said, E> {
        Ok(Said(format!("str:{s}")))
    }

    fn visit_string<E: Error>(self, s: String) -> Result<Said, E> {
        Ok(Said(format!("owned:{s}")))
    }

    fn visit_borrowed_str<E: Error>(self, s: &'de str) -> Result<Said, E> {
        Ok(Said(format!("borrowed:{s}")))
    }
}

impl<'de> Deserialize<'de> for Said {
    fn deserialize<D: Deserializer<'de>>(d: D) -> Result<Self, D::Error> {
        d.deserialize_string(SaidVisitor)
    }
}

/// The same, read by 'deserialize_any'.
#[derive(Debug)]
pub struct AnySaid(Said);

impl<'de> Deserialize<'de> for AnySaid {
    fn deserialize<D: Deserializer<'de>>(d: D) -> Result<Self, D::Error> {
        d.deserialize_any(SaidVisitor).map(AnySaid)
    }
}

/// The same, read by 'deserialize_str', whose visitor has 'visit_string' alone
/// of the owned kind: serde's 'visit_str' forwards nothing to it.
#[derive(Debug)]
pub struct StrSaid(Said);

impl<'de> Deserialize<'de> for StrSaid {
    fn deserialize<D: Deserializer<'de>>(d: D) -> Result<Self, D::Error> {
        d.deserialize_str(SaidVisitor).map(StrSaid)
    }
}

/// Of 'visit_string' alone: 'visit_str''s default is an error.
#[derive(Debug)]
pub struct Owned(String);

struct OwnedVisitor;

impl<'de> Visitor<'de> for OwnedVisitor {
    type Value = Owned;

    fn expecting(&self, f: &mut fmt::Formatter) -> fmt::Result {
        f.write_str("an owned string")
    }

    fn visit_string<E: Error>(self, s: String) -> Result<Owned, E> {
        Ok(Owned(s))
    }
}

impl<'de> Deserialize<'de> for Owned {
    fn deserialize<D: Deserializer<'de>>(d: D) -> Result<Self, D::Error> {
        d.deserialize_string(OwnedVisitor)
    }
}

#[derive(Debug, Deserialize)]
pub struct Fields {
    a: Said,
    b: AnySaid,
    c: StrSaid,
}

#[derive(Debug, Deserialize)]
#[serde(untagged)]
pub enum Untagged {
    One(Said),
}

#[derive(Debug, Deserialize)]
#[serde(tag = "t")]
pub enum Tagged {
    V { s: Said, any: AnySaid },
}

fn read<T: for<'de> Deserialize<'de> + fmt::Debug>(text: &str) -> String {
    let from_text = serde_json::from_str::<T>(text);
    let value: serde_json::Value = serde_json::from_str(text).unwrap();
    let from_value = serde_json::from_value::<T>(value);
    format!("{from_text:?}; {from_value:?}")
}

pub fn report() -> String {
    [
        read::<Said>(r#""hi""#),
        read::<Said>(r#""h\ni""#),
        read::<AnySaid>(r#""hi""#),
        read::<AnySaid>(r#""h\ni""#),
        read::<StrSaid>(r#""hi""#),
        read::<StrSaid>(r#""h\ni""#),
        read::<Owned>(r#""hi""#),
        read::<Fields>(r#"{"a": "x", "b": "y\n", "c": "z"}"#),
        read::<Untagged>(r#""u""#),
        read::<Untagged>(r#""u\n""#),
        read::<Tagged>(r#"{"t": "V", "s": "p", "any": "q\n"}"#),
    ]
    .join("\n")
}
`);
  const expected = JSON.parse(run([native(dir)]));
  expect((await compiled(dir)).report()).toBe(expected);
});

// A 128-bit integer a hand-written impl writes, `serialize_u128` or
// `serialize_i128`, by each serializer's own rule (#1547): text writes its
// digits; a `Value` holds it only where it fits in an `i64` or a `u64`,
// else it's serde_json's "number out of range"; and serde's flattening and
// tagging serializers, which don't override the trait's default, give its
// "u128 is not supported". Within a flattened map it's the object's writer's.
test("serde: a 128-bit integer is written by each serializer's own rule", async () => {
  const dir = fixture("serde-protocol-128-bit");
  writeFileSync(join(dir, "cases.rs"), String.raw`
use serde::ser::SerializeSeq;
use serde::{Serialize, Serializer};

/// Writes itself by 'serialize_u128'.
pub struct Wide(pub u128);

impl Serialize for Wide {
    fn serialize<S: Serializer>(&self, s: S) -> Result<S::Ok, S::Error> {
        s.serialize_u128(self.0)
    }
}

/// Writes itself by 'serialize_i128'.
pub struct Signed(pub i128);

impl Serialize for Signed {
    fn serialize<S: Serializer>(&self, s: S) -> Result<S::Ok, S::Error> {
        s.serialize_i128(self.0)
    }
}

/// A sequence of both, an error asked and written on after.
pub struct Both(pub u128, pub i128);

impl Serialize for Both {
    fn serialize<S: Serializer>(&self, s: S) -> Result<S::Ok, S::Error> {
        let mut seq = s.serialize_seq(None)?;
        let wide = seq.serialize_element(&Wide(self.0)).is_err();
        seq.serialize_element(&Signed(self.1))?;
        seq.serialize_element(&wide)?;
        seq.end()
    }
}

/// The 64-bit ones, beside.
pub struct Narrow(pub u64, pub i64);

impl Serialize for Narrow {
    fn serialize<S: Serializer>(&self, s: S) -> Result<S::Ok, S::Error> {
        let mut seq = s.serialize_seq(Some(2))?;
        seq.serialize_element(&self.0)?;
        seq.serialize_element(&self.1)?;
        seq.end()
    }
}

#[derive(Serialize)]
#[serde(untagged)]
pub enum Either {
    W(Wide),
}

#[derive(Serialize)]
pub struct Flat {
    #[serde(flatten)]
    w: Wide,
}

#[derive(Serialize)]
pub struct FlatSigned {
    #[serde(flatten)]
    s: Signed,
}

/// A map of one, flattened: its value written by the object's own writer.
pub struct InMap(pub u128);

impl Serialize for InMap {
    fn serialize<S: Serializer>(&self, s: S) -> Result<S::Ok, S::Error> {
        use serde::ser::SerializeMap;
        let mut map = s.serialize_map(None)?;
        map.serialize_entry("w", &Wide(self.0))?;
        map.end()
    }
}

#[derive(Serialize)]
pub struct FlatMap {
    a: u8,
    #[serde(flatten)]
    m: InMap,
}

fn both<T: Serialize>(value: &T) -> String {
    format!("{:?}; {:?}", serde_json::to_string(value), serde_json::to_value(value))
}

pub fn report() -> String {
    [
        both(&Wide(u128::MAX)),
        both(&Wide(u64::MAX as u128)),
        both(&Wide(u64::MAX as u128 + 1)),
        both(&Wide(0)),
        both(&Signed(i128::MIN)),
        both(&Signed(i128::MAX)),
        both(&Signed(i64::MIN as i128)),
        both(&Signed(i64::MIN as i128 - 1)),
        both(&Signed(-5)),
        both(&Signed(5)),
        both(&Signed(u64::MAX as i128)),
        both(&Signed(u64::MAX as i128 + 1)),
        both(&Both(u128::MAX, -1)),
        both(&Both(1, i128::MAX)),
        both(&Narrow(u64::MAX, i64::MIN)),
        both(&vec![Wide(1), Wide(u128::MAX)]),
        both(&Either::W(Wide(u128::MAX))),
        both(&Either::W(Wide(3))),
        both(&Flat { w: Wide(3) }),
        both(&FlatSigned { s: Signed(-3) }),
        both(&FlatMap { a: 1, m: InMap(3) }),
        both(&FlatMap { a: 1, m: InMap(u128::MAX) }),
    ]
    .join("\n")
}
`);
  const expected = JSON.parse(run([native(dir)]));
  expect((await compiled(dir)).report()).toBe(expected);
});

// A hand-written visitor's 'deserialize_option', by each reader's own rule
// (#1597, #1549): the text's and a 'Value's give 'null' to its
// 'visit_none' and anything else to its 'visit_some'; serde's buffered
// content, an untagged or an internally tagged enum's, gives 'null', a
// unit, to its 'visit_unit'; a missing field is its 'visit_none'; and a
// flattened one can't be read. Of a method it lacks, serde's error.
test("serde: a visitor's deserialize_option is each reader's own", async () => {
  const dir = fixture("serde-protocol-option-visitor");
  writeFileSync(join(dir, "cases.rs"), String.raw`
use serde::de::{Error, Visitor};
use serde::{Deserialize, Deserializer};
use std::fmt;

/// Read by 'deserialize_option': 'visit_none' and 'visit_some'.
#[derive(Debug)]
pub struct Answer(u32);

struct AnswerVisitor;

impl<'de> Visitor<'de> for AnswerVisitor {
    type Value = Answer;
    fn expecting(&self, f: &mut fmt::Formatter) -> fmt::Result {
        f.write_str("an option")
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
        d.deserialize_option(AnswerVisitor)
    }
}

/// The same with 'visit_unit' too.
#[derive(Debug)]
pub struct WithUnit(u32);

struct WithUnitVisitor;

impl<'de> Visitor<'de> for WithUnitVisitor {
    type Value = WithUnit;
    fn expecting(&self, f: &mut fmt::Formatter) -> fmt::Result {
        f.write_str("an option or a unit")
    }
    fn visit_none<E: Error>(self) -> Result<WithUnit, E> {
        Ok(WithUnit(0))
    }
    fn visit_unit<E: Error>(self) -> Result<WithUnit, E> {
        Ok(WithUnit(1))
    }
    fn visit_some<D: Deserializer<'de>>(self, d: D) -> Result<WithUnit, D::Error> {
        u32::deserialize(d).map(WithUnit)
    }
}

impl<'de> Deserialize<'de> for WithUnit {
    fn deserialize<D: Deserializer<'de>>(d: D) -> Result<Self, D::Error> {
        d.deserialize_option(WithUnitVisitor)
    }
}

/// Only 'visit_some': a 'null' is serde's invalid type.
#[derive(Debug)]
pub struct SomeOnly(u32);

struct SomeOnlyVisitor;

impl<'de> Visitor<'de> for SomeOnlyVisitor {
    type Value = SomeOnly;
    fn expecting(&self, f: &mut fmt::Formatter) -> fmt::Result {
        f.write_str("some")
    }
    fn visit_some<D: Deserializer<'de>>(self, d: D) -> Result<SomeOnly, D::Error> {
        u32::deserialize(d).map(SomeOnly)
    }
}

impl<'de> Deserialize<'de> for SomeOnly {
    fn deserialize<D: Deserializer<'de>>(d: D) -> Result<Self, D::Error> {
        d.deserialize_option(SomeOnlyVisitor)
    }
}

#[derive(Debug, Deserialize)]
#[serde(untagged)]
pub enum Untagged {
    A { a: Answer },
}

#[derive(Debug, Deserialize)]
#[serde(untagged)]
pub enum UntaggedUnit {
    W { w: WithUnit },
}

#[derive(Debug, Deserialize)]
#[serde(tag = "t")]
pub enum Tagged {
    V { a: Answer, w: WithUnit },
}

#[derive(Debug, Deserialize)]
pub struct Fields {
    a: Answer,
    s: SomeOnly,
}

#[derive(Debug, Deserialize)]
pub struct Flat {
    n: u8,
    #[serde(flatten)]
    a: Answer,
}

fn read<T: for<'de> Deserialize<'de> + fmt::Debug>(text: &str) -> String {
    let from_text = serde_json::from_str::<T>(text);
    let value: serde_json::Value = serde_json::from_str(text).unwrap();
    let from_value = serde_json::from_value::<T>(value);
    format!("{from_text:?}; {from_value:?}")
}

pub fn report() -> String {
    [
        read::<Answer>("null"),
        read::<Answer>("5"),
        read::<Answer>("\"x\""),
        read::<WithUnit>("null"),
        read::<WithUnit>("7"),
        read::<SomeOnly>("null"),
        read::<SomeOnly>("3"),
        read::<Untagged>(r#"{"a": null}"#),
        read::<Untagged>(r#"{"a": 4}"#),
        read::<UntaggedUnit>(r#"{"w": null}"#),
        read::<UntaggedUnit>(r#"{"w": 2}"#),
        read::<Tagged>(r#"{"t": "V", "a": null, "w": null}"#),
        read::<Tagged>(r#"{"t": "V", "a": 1, "w": 2}"#),
        read::<Fields>(r#"{"a": null, "s": 1}"#),
        read::<Fields>(r#"{"a": 2, "s": null}"#),
        read::<Vec<Answer>>("[null, 1]"),
        read::<Fields>(r#"{"s": 1}"#),
        read::<Flat>(r#"{"n": 1}"#),
        read::<Flat>(r#"{"n": 1, "x": 2}"#),
    ]
    .join("\n")
}
`);
  const expected = JSON.parse(run([native(dir)]));
  expect((await compiled(dir)).report()).toBe(expected);
});
