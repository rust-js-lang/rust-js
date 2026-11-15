// What a library keeps, a constant, a cell's value, is handed out as a copy
// of its own, whoever changes it (#1907): two libraries joined only by JS,
// one's `origin()` given to the other's `shift`, which changes it in place,
// leave the first's constant as Rust does. The three crates' own rows are
// test/crates/triples/library_value_changed_by_another_library.
import { beforeAll, expect, test } from "bun:test";
import { readFileSync, writeFileSync } from "node:fs";
import { join } from "node:path";
import { chain, triples } from "./crates";
import { buildReact, compiler, fixture, run, target } from "./support";
import { underLoad } from "../scripts/timing";

beforeAll(buildReact, 600_000);

test("a library's kept value given by JS to another library that changes it is a copy of its own", async () => {
  const out = chain(triples, "library_value_changed_by_another_library", ["dep", "mid", "app"]);
  const dep = await import(join(out, "dep", "lib.js"));
  const mid = await import(join(out, "mid", "lib.js"));
  const origin = dep.origin();
  mid.shift(origin);
  expect([origin, dep.origin()]).toEqual([{ x: 0, y: 10 }, { x: 0, y: 0 }]);
  // A cell's value, read through the `Rc` JS holds.
  const store = dep.store();
  const read = dep.read(store);
  mid.shift(read);
  expect([read, dep.read(store)]).toEqual([{ x: 0, y: 10 }, { x: 0, y: 0 }]);
  // What it stores, of which only a number leaves: as it is.
  const depJs = readFileSync(join(out, "dep", "lib.js"), "utf8");
  expect(depJs.slice(depJs.indexOf("export function gotN")).split("\n}")[0]).not.toContain("...");
  // And `mid`'s, given by JS a function of `dep`'s to call.
  expect([mid.withSpot(dep.origin), dep.origin()]).toEqual([{ x: 0, y: 3 }, { x: 0, y: 0 }]);
}, underLoad(300_000));

// What a library stores, given to a WebIDL operation's dictionary parameter,
// which the platform reads into an IDL value and never keeps or changes
// (WebIDL's conversion to a dictionary type): as it is. Given to an `any`
// one, `postMessage`'s message, which may be kept: a copy (ADR 0262, case
// none, of the first; #1907).
test("a library's stored dictionary given to WebIDL is as it is, and its stored value given as `any` a copy", () => {
  const dir = fixture("library-kept-dictionary");
  writeFileSync(join(dir, "lib.rs"), `use js::StructuredClone;
use webapi::window_global::window;
use webapi::{TextDecodeOptions, TextDecoder};

#[derive(Clone, Copy)]
pub struct Saved {
    pub count: i32,
}

// SAFETY: a struct of a number, which the structured clone copies.
unsafe impl StructuredClone for Saved {}

pub const SAVED: Saved = Saved { count: 1 };

pub const STREAMING: TextDecodeOptions = TextDecodeOptions { stream: Some(true) };

pub fn decoded(decoder: &TextDecoder) -> String {
    decoder.decode(None, Some(STREAMING))
}

pub fn posted() {
    window().post_message(SAVED, None);
}
`);
  run([compiler, join(dir, "lib.rs"), "-o", join(dir, "out", "lib.js"), "--library", "--manifest", join(dir, "out", "lib.manifest.json"),
    "--", "--crate-name", "kept", "--crate-type=lib", "--edition=2024",
    "--extern", `webapi=${join(target, "libwebapi.rmeta")}`, "--extern", `js=${join(target, "libjs.rmeta")}`, "-L", target]);
  const js = readFileSync(join(dir, "out", "lib.js"), "utf8");
  expect(js).toContain("decoder.decode(undefined, STREAMING)");
  expect(js).toContain("postMessage({ ...SAVED })");
}, underLoad(300_000));
