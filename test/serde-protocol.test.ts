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
