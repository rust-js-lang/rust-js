// std's types whose serde impl is serde's own (#1623): each row of the
// catalog, `src/lower/serde/std_forms.rs`, written and read against native
// serde_json, the oracle, its errors among them, by family.
import { beforeAll, expect, test } from "bun:test";
import { writeFileSync } from "node:fs";
import { join } from "node:path";
import { compiled, native } from "./serde-support";
import { buildCompiler, buildSerde, fixture, run } from "./support";

beforeAll(() => {
  buildCompiler();
  buildSerde("rlib");
}, 600_000);

/** `report()`'s lines, each a value written, or a text read as `T`: what it
 * gives, `Debug`, or its error. */
const harness = String.raw`
fn read<T: serde::de::DeserializeOwned + std::fmt::Debug>(out: &mut String, text: &str) {
    match serde_json::from_str::<T>(text) {
        Ok(v) => out.push_str(&format!("{text} -> {v:?}\n")),
        Err(e) => out.push_str(&format!("{text} -> error: {e}\n")),
    }
}
fn read_value<T: serde::de::DeserializeOwned + std::fmt::Debug>(out: &mut String, value: serde_json::Value) {
    match serde_json::from_value::<T>(value.clone()) {
        Ok(v) => out.push_str(&format!("{value} as value -> {v:?}\n")),
        Err(e) => out.push_str(&format!("{value} as value -> error: {e}\n")),
    }
}
fn write<T: serde::Serialize>(out: &mut String, value: &T) {
    out.push_str(&serde_json::to_string(value).unwrap());
    out.push(' ');
    out.push_str(&serde_json::to_string_pretty(value).unwrap().replace('\n', "|"));
    out.push(' ');
    out.push_str(&serde_json::to_value(value).unwrap().to_string());
    out.push('\n');
}
`;

// Ranges and bounds: `Range` and `RangeInclusive` as `{"start","end"}`,
// read from a map or a sequence, an unknown, missing or duplicate field
// serde's error; `Bound` externally tagged.
test("serde: a range and a bound are written and read as serde's own impls do", async () => {
  const dir = fixture("serde-std-ranges");
  writeFileSync(join(dir, "cases.rs"), harness + String.raw`
use std::ops::{Bound, Range, RangeInclusive};

#[derive(Debug, serde::Serialize, serde::Deserialize)]
pub struct Window {
    pub rows: Range<u32>,
    pub cols: RangeInclusive<i64>,
    pub low: Bound<String>,
    pub high: Bound<f64>,
}

pub fn report() -> String {
    let mut out = String::new();
    write(&mut out, &(1u32..5));
    write(&mut out, &(-3i64..=3));
    write(&mut out, &(5u8..1));
    write(&mut out, &vec![Bound::Unbounded, Bound::Included(3u16), Bound::Excluded(4)]);
    let window = Window { rows: 0..10, cols: -1..=1, low: Bound::Included("a".into()), high: Bound::Unbounded };
    write(&mut out, &window);
    let mut stepped = 1u32..=2;
    stepped.next();
    stepped.next();
    write(&mut out, &stepped);
    for text in [
        r#"{"start":1,"end":5}"#,
        r#"{"end":5,"start":1}"#,
        "[2,7]",
        "[2]",
        "[2,7,9]",
        r#"{"start":1}"#,
        r#"{"start":1,"start":2,"end":3}"#,
        r#"{"start":1,"end":2,"step":3}"#,
        r#"{"start":-1,"end":2}"#,
        "null",
        "3",
    ] {
        read::<Range<u32>>(&mut out, text);
        read::<RangeInclusive<u32>>(&mut out, text);
    }
    for text in [
        r#""Unbounded""#,
        r#"{"Included":3}"#,
        r#"{"Excluded":-2}"#,
        r#""Included""#,
        r#"{"Unbounded":null}"#,
        r#"{"Sideways":1}"#,
        r#"{"Included":3,"Excluded":4}"#,
        "[]",
        "0",
    ] {
        read::<Bound<i32>>(&mut out, text);
    }
    read::<Window>(&mut out, r#"{"rows":{"start":0,"end":2},"cols":[1,1],"low":"Unbounded","high":{"Excluded":2.5}}"#);
    let back: Window = serde_json::from_str(&serde_json::to_string(&window).unwrap()).unwrap();
    out.push_str(&format!("{back:?}\n"));
    read_value::<Range<u8>>(&mut out, serde_json::json!({"start": 1, "end": 2}));
    read_value::<Range<u8>>(&mut out, serde_json::json!([3, 4]));
    read_value::<Range<u8>>(&mut out, serde_json::json!({"start": 1, "stop": 2}));
    read_value::<Bound<u8>>(&mut out, serde_json::json!({"Included": 7}));
    read_value::<Bound<u8>>(&mut out, serde_json::json!("Nowhere"));
    out
}
`);
  const expected = JSON.parse(run([native(dir)]));
  expect((await compiled(dir)).report()).toBe(expected);
});

// Time: a `Duration` as `{"secs","nanos"}`, its nanoseconds carried where
// they're a second or more, and serde's `overflow deserializing Duration`
// where they carry past `u64::MAX` seconds; a `SystemTime` as its time since
// the epoch, serde's error of one before it, and of an offset unix's
// `SystemTime` can't hold.
test("serde: a Duration and a SystemTime are written and read as serde's own impls do", async () => {
  const dir = fixture("serde-std-time");
  writeFileSync(join(dir, "cases.rs"), harness + String.raw`
use std::time::{Duration, SystemTime, UNIX_EPOCH};

#[derive(Debug, serde::Serialize, serde::Deserialize)]
pub struct Timing {
    pub took: Duration,
    pub at: SystemTime,
    pub every: Vec<Duration>,
}

fn write_result<T: serde::Serialize>(out: &mut String, value: &T) {
    match serde_json::to_string(value) {
        Ok(text) => out.push_str(&format!("{text}\n")),
        Err(e) => out.push_str(&format!("error: {e}\n")),
    }
    match serde_json::to_value(value) {
        Ok(v) => out.push_str(&format!("{v}\n")),
        Err(e) => out.push_str(&format!("error: {e}\n")),
    }
}

pub fn report() -> String {
    let mut out = String::new();
    write(&mut out, &Duration::new(5, 30));
    write(&mut out, &Duration::from_millis(1500));
    write(&mut out, &Duration::MAX);
    write(&mut out, &Duration::ZERO);
    let at = UNIX_EPOCH + Duration::new(1_700_000_000, 123_456_789);
    write(&mut out, &at);
    write(&mut out, &UNIX_EPOCH);
    let timing = Timing { took: Duration::from_micros(2500), at, every: vec![Duration::from_secs(1), Duration::new(0, 1)] };
    write(&mut out, &timing);
    write_result(&mut out, &(UNIX_EPOCH - Duration::from_secs(1)));
    write_result(&mut out, &vec![Some(UNIX_EPOCH - Duration::from_nanos(1))]);
    for text in [
        r#"{"secs":1,"nanos":2}"#,
        r#"{"nanos":2,"secs":1}"#,
        "[3,4]",
        r#"{"secs":1,"nanos":2000000000}"#,
        r#"{"secs":1,"nanos":4294967295}"#,
        r#"{"secs":18446744073709551615,"nanos":999999999}"#,
        r#"{"secs":18446744073709551615,"nanos":1000000000}"#,
        "[18446744073709551614,2000000000]",
        r#"{"secs":1}"#,
        r#"{"secs":1,"nanos":2,"millis":3}"#,
        r#"{"secs":1,"secs":2,"nanos":3}"#,
        r#"{"secs":-1,"nanos":0}"#,
        r#"{"secs":1,"nanos":4294967296}"#,
        "[1]",
        "1.5",
    ] {
        read::<Duration>(&mut out, text);
    }
    for text in [
        r#"{"secs_since_epoch":1700000000,"nanos_since_epoch":5}"#,
        "[0,0]",
        r#"{"secs_since_epoch":1,"nanos_since_epoch":1500000000}"#,
        r#"{"secs_since_epoch":9223372036854775807,"nanos_since_epoch":999999999}"#,
        r#"{"secs_since_epoch":9223372036854775807,"nanos_since_epoch":1000000000}"#,
        r#"{"secs_since_epoch":9223372036854775808,"nanos_since_epoch":0}"#,
        r#"{"secs_since_epoch":18446744073709551615,"nanos_since_epoch":1000000000}"#,
        r#"{"secs":1,"nanos":2}"#,
        r#"{"secs_since_epoch":1}"#,
    ] {
        match serde_json::from_str::<SystemTime>(text) {
            Ok(t) => out.push_str(&format!("{text} -> {:?}\n", t.duration_since(UNIX_EPOCH))),
            Err(e) => out.push_str(&format!("{text} -> error: {e}\n")),
        }
    }
    let back: Timing = serde_json::from_str(&serde_json::to_string(&timing).unwrap()).unwrap();
    out.push_str(&format!("{:?} {:?} {:?}\n", back.took, back.at.duration_since(UNIX_EPOCH), back.every));
    read_value::<Duration>(&mut out, serde_json::json!({"secs": 7, "nanos": 8}));
    read_value::<Duration>(&mut out, serde_json::json!([u64::MAX, 2_000_000_000u32]));
    read_value::<Duration>(&mut out, serde_json::json!({"secs": 7}));
    out
}
`);
  const expected = JSON.parse(run([native(dir)]));
  expect((await compiled(dir)).report()).toBe(expected);
});

// Wrappers: a `Wrapping` and a `Reverse` as what they hold, both ways, a
// read's error that of what they hold; a `PhantomData` as `null`, read as
// serde's unit.
test("serde: a Wrapping, a Reverse and a PhantomData are written and read as serde's own impls do", async () => {
  const dir = fixture("serde-std-wrappers");
  writeFileSync(join(dir, "cases.rs"), harness + String.raw`
use std::cmp::Reverse;
use std::marker::PhantomData;
use std::num::Wrapping;

#[derive(Debug, serde::Serialize, serde::Deserialize)]
pub struct Marked {
    pub count: Wrapping<u8>,
    pub order: Vec<Reverse<i32>>,
    pub nested: Reverse<Wrapping<u16>>,
    pub kind: PhantomData<String>,
}

pub fn report() -> String {
    let mut out = String::new();
    write(&mut out, &Wrapping(250u8));
    write(&mut out, &Reverse("text"));
    write(&mut out, &PhantomData::<u8>);
    let marked = Marked { count: Wrapping(255u8) + Wrapping(2u8), order: vec![Reverse(3), Reverse(-1)], nested: Reverse(Wrapping(9)), kind: PhantomData };
    write(&mut out, &marked);
    for text in ["7", "256", "-1", "\"7\"", "null"] {
        read::<Wrapping<u8>>(&mut out, text);
        read::<Reverse<u8>>(&mut out, text);
    }
    for text in ["null", "1", "{}", "[]", "\"PhantomData\""] {
        read::<PhantomData<u8>>(&mut out, text);
    }
    read::<Marked>(&mut out, r#"{"count":4,"order":[1,2],"nested":5,"kind":null}"#);
    read::<Marked>(&mut out, r#"{"count":4,"order":[1,2],"nested":5,"kind":0}"#);
    read::<Marked>(&mut out, r#"{"count":4,"order":[1,2],"nested":5}"#);
    let back: Marked = serde_json::from_str(&serde_json::to_string(&marked).unwrap()).unwrap();
    out.push_str(&format!("{back:?}\n"));
    read_value::<Reverse<String>>(&mut out, serde_json::json!("v"));
    read_value::<PhantomData<u8>>(&mut out, serde_json::json!(null));
    read_value::<PhantomData<u8>>(&mut out, serde_json::json!(false));
    out
}
`);
  const expected = JSON.parse(run([native(dir)]));
  expect((await compiled(dir)).report()).toBe(expected);
});

// Cells and locks: what each holds, read through it as serde's impl does,
// `get()`, `try_borrow()`, `lock()` and `read()`: serde's `already mutably
// borrowed` while a `RefCell` is mutably borrowed, and `lock poison error
// while serializing` of a `Mutex` a caught panic poisoned; read as `::new`
// of what's read.
test("serde: a Cell, a RefCell, a Mutex and an RwLock are written and read as serde's own impls do", async () => {
  const dir = fixture("serde-std-cells");
  writeFileSync(join(dir, "cases.rs"), harness + String.raw`
use std::cell::{Cell, RefCell};
use std::rc::Rc;
use std::sync::{Mutex, RwLock};

#[derive(Debug, serde::Serialize, serde::Deserialize)]
pub struct Shared {
    pub hits: Cell<u32>,
    pub names: RefCell<Vec<String>>,
    pub guarded: Mutex<i64>,
    pub read_mostly: RwLock<String>,
}

fn write_result<T: serde::Serialize>(out: &mut String, value: &T) {
    match serde_json::to_string(value) {
        Ok(text) => out.push_str(&format!("{text}\n")),
        Err(e) => out.push_str(&format!("error: {e}\n")),
    }
}

/// Panics as it's written; and gives a write's error.
pub struct Panics;
impl serde::Serialize for Panics {
    fn serialize<S: serde::Serializer>(&self, _s: S) -> Result<S::Ok, S::Error> {
        panic!("written")
    }
}
pub struct Fails;
impl serde::Serialize for Fails {
    fn serialize<S: serde::Serializer>(&self, _s: S) -> Result<S::Ok, S::Error> {
        Err(serde::ser::Error::custom("fails"))
    }
}

pub fn report() -> String {
    let mut out = String::new();
    let panicking = Mutex::new(Panics);
    let caught = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| serde_json::to_string(&panicking)));
    out.push_str(&format!("panicked {} poisoned {}\n", caught.is_err(), panicking.is_poisoned()));
    let failing = Mutex::new(Fails);
    write_result(&mut out, &failing);
    out.push_str(&format!("poisoned {}\n", failing.is_poisoned()));
    let reading = RwLock::new(Panics);
    let caught = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| serde_json::to_string(&reading)));
    out.push_str(&format!("panicked {} poisoned {}\n", caught.is_err(), reading.is_poisoned()));
    let shared = Shared {
        hits: Cell::new(2),
        names: RefCell::new(vec!["a".into()]),
        guarded: Mutex::new(-5),
        read_mostly: RwLock::new("r".into()),
    };
    write(&mut out, &shared);
    shared.hits.set(3);
    shared.names.borrow_mut().push("b".into());
    write(&mut out, &shared);
    {
        let _held = shared.names.borrow();
        write_result(&mut out, &shared);
    }
    {
        let _held = shared.names.borrow_mut();
        write_result(&mut out, &shared);
        write_result(&mut out, &vec![1u8]);
    }
    let poisoned = Mutex::new(7u8);
    let caught = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
        let _guard = poisoned.lock().unwrap();
        panic!("poison");
    }));
    out.push_str(&format!("caught {}\n", caught.is_err()));
    write_result(&mut out, &poisoned);
    write_result(&mut out, &(1u8, &poisoned));
    let shared_cell = Rc::new(RefCell::new(vec![1u8, 2]));
    let other = Rc::clone(&shared_cell);
    other.borrow_mut().push(3);
    write(&mut out, &*shared_cell);
    for text in [
        r#"{"hits":1,"names":["x"],"guarded":2,"read_mostly":"s"}"#,
        r#"{"hits":-1,"names":[],"guarded":2,"read_mostly":"s"}"#,
        r#"{"hits":1,"names":"x","guarded":2,"read_mostly":"s"}"#,
    ] {
        match serde_json::from_str::<Shared>(text) {
            Ok(s) => {
                s.names.borrow_mut().push("added".into());
                *s.guarded.lock().unwrap() += 1;
                out.push_str(&format!("{text} -> {} {:?} {} {}\n", s.hits.get(), s.names.borrow(), s.guarded.lock().unwrap(), s.read_mostly.read().unwrap()));
            }
            Err(e) => out.push_str(&format!("{text} -> error: {e}\n")),
        }
    }
    read::<Cell<u8>>(&mut out, "4");
    read::<RefCell<u8>>(&mut out, "300");
    read::<Mutex<String>>(&mut out, "\"m\"");
    read_value::<RwLock<bool>>(&mut out, serde_json::json!(true));
    out
}
`);
  const expected = JSON.parse(run([native(dir)]));
  expect((await compiled(dir)).report()).toBe(expected);
});

// Numbers: an `i128` and a `u128` written as their digits, and to a `Value`
// only where they fit in 64 bits, else serde_json's `number out of range`;
// read by serde_json's own scan of their digits, whole, never a float; and
// a `Saturating` read by serde's visitor, which clamps any integer.
test("serde: an i128, a u128 and a Saturating are written and read as serde_json does", async () => {
  const dir = fixture("serde-std-numbers");
  writeFileSync(join(dir, "cases.rs"), harness + String.raw`
use std::num::Saturating;

#[derive(Debug, serde::Serialize, serde::Deserialize)]
pub struct Wide {
    pub big: i128,
    pub huge: u128,
    pub clamped: Saturating<u8>,
    pub signed: Saturating<i16>,
}

fn write_result<T: serde::Serialize>(out: &mut String, value: &T) {
    match serde_json::to_string(value) {
        Ok(text) => out.push_str(&format!("{text}\n")),
        Err(e) => out.push_str(&format!("error: {e}\n")),
    }
    match serde_json::to_value(value) {
        Ok(v) => out.push_str(&format!("{v}\n")),
        Err(e) => out.push_str(&format!("error: {e}\n")),
    }
}

pub fn report() -> String {
    let mut out = String::new();
    for v in [0i128, -1, i64::MIN as i128, i64::MIN as i128 - 1, u64::MAX as i128, u64::MAX as i128 + 1, i128::MIN, i128::MAX] {
        write_result(&mut out, &v);
    }
    for v in [0u128, u64::MAX as u128, u64::MAX as u128 + 1, u128::MAX] {
        write_result(&mut out, &v);
    }
    write_result(&mut out, &Saturating(250u8));
    let wide = Wide { big: -170141183460469231731687303715884105728, huge: 340282366920938463463374607431768211455, clamped: Saturating(7), signed: Saturating(-3) };
    write_result(&mut out, &wide);
    for text in [
        "0", "-0", "12", "-12", "170141183460469231731687303715884105727", "170141183460469231731687303715884105728",
        "-170141183460469231731687303715884105728", "-170141183460469231731687303715884105729",
        "1.5", "1e3", "00", "01", "-", "\"5\"", "null", "true", " 7 ", "[1]",
        "340282366920938463463374607431768211455", "340282366920938463463374607431768211456",
    ] {
        read::<i128>(&mut out, text);
        read::<u128>(&mut out, text);
    }
    for text in ["300", "-5", "255", "18446744073709551615", "-9223372036854775808", "1.0", "\"3\"", "null"] {
        read::<Saturating<u8>>(&mut out, text);
        read::<Saturating<i16>>(&mut out, text);
        read::<Saturating<i64>>(&mut out, text);
    }
    read::<Wide>(&mut out, r#"{"big":-5,"huge":99999999999999999999999,"clamped":999,"signed":-40000}"#);
    read::<Wide>(&mut out, r#"{"big":1.5,"huge":1,"clamped":1,"signed":1}"#);
    read::<Vec<u128>>(&mut out, "[1, 2,3]");
    let keyed: std::collections::BTreeMap<i128, u8> = [(-1i128, 1u8), (i128::MAX, 2)].into_iter().collect();
    write_result(&mut out, &keyed);
    read::<std::collections::BTreeMap<i128, u8>>(&mut out, r#"{"-1":1,"170141183460469231731687303715884105727":2}"#);
    read::<std::collections::BTreeMap<u128, u8>>(&mut out, r#"{"-1":1}"#);
    read_value::<i128>(&mut out, serde_json::json!(-7));
    read_value::<u128>(&mut out, serde_json::json!(-7));
    read_value::<u128>(&mut out, serde_json::json!(18446744073709551615u64));
    read_value::<i128>(&mut out, serde_json::json!(2.5));
    read_value::<Saturating<u8>>(&mut out, serde_json::json!(1000));
    out
}
`);
  const expected = JSON.parse(run([native(dir)]));
  expect((await compiled(dir)).report()).toBe(expected);
});

// Paths and strings: a `PathBuf` and a `Path` as their text, a `PathBuf`
// read as serde's `path string`; a `Cow` as what it borrows or owns, read as
// `Cow::Owned`; a `Box<str>` and a `Box<[T]>` read as a `String` and a `Vec`.
test("serde: a path, a Cow and a boxed str or slice are written and read as serde's own impls do", async () => {
  const dir = fixture("serde-std-paths");
  writeFileSync(join(dir, "cases.rs"), harness + String.raw`
use std::borrow::Cow;
use std::path::{Path, PathBuf};

#[derive(Debug, serde::Serialize, serde::Deserialize)]
pub struct Entry<'a> {
    pub at: PathBuf,
    pub label: Cow<'a, str>,
    pub sizes: Cow<'a, [u32]>,
    pub note: Box<str>,
    pub parts: Box<[i8]>,
}

pub fn report() -> String {
    let mut out = String::new();
    write(&mut out, &PathBuf::from("a/b c\"d"));
    write(&mut out, &Path::new("/tmp/x"));
    write(&mut out, &Cow::Borrowed("cow"));
    write(&mut out, &Cow::<str>::Owned("owned".to_string()));
    write(&mut out, &Cow::Borrowed(&[1u8, 2][..]));
    let entry = Entry { at: PathBuf::from("dir/file.txt"), label: Cow::Borrowed("l"), sizes: Cow::Owned(vec![3, 4]), note: "n".into(), parts: vec![-1, 2].into_boxed_slice() };
    write(&mut out, &entry);
    for text in ["\"p/q\"", "\"\"", "\"esc\\naped\"", "1", "null", "[\"a\"]"] {
        read::<PathBuf>(&mut out, text);
        read::<Cow<str>>(&mut out, text);
        read::<Box<str>>(&mut out, text);
    }
    for text in ["[1,2]", "[]", "[1,\"x\"]", "\"s\"", "{}"] {
        read::<Cow<[u32]>>(&mut out, text);
        read::<Box<[i8]>>(&mut out, text);
    }
    match serde_json::from_str::<Entry>(r#"{"at":"x/y","label":"lab","sizes":[5],"note":"nt","parts":[7]}"#) {
        Ok(mut e) => {
            e.label.to_mut().push('!');
            e.at.push("z");
            out.push_str(&format!("{e:?} {}\n", matches!(e.label, Cow::Owned(_))));
        }
        Err(e) => out.push_str(&format!("error: {e}\n")),
    }
    read_value::<PathBuf>(&mut out, serde_json::json!("v/w"));
    read_value::<PathBuf>(&mut out, serde_json::json!(5));
    read_value::<Cow<str>>(&mut out, serde_json::json!("cv"));
    read_value::<Box<[i8]>>(&mut out, serde_json::json!([1, 300]));
    out
}
`);
  const expected = JSON.parse(run([native(dir)]));
  expect((await compiled(dir)).report()).toBe(expected);
});
