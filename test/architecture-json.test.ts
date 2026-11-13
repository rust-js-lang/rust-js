// The JSON readers' rules of the architecture: serde's `Deserializer` is
// one backend interface, which the text's reader, a `Value`'s and serde's
// buffered content each write (#1549, ADR 0459).
import { expect, test } from "bun:test";
import { readFileSync } from "node:fs";
import { join } from "node:path";
import { root } from "./support";

const read = (path: string) => readFileSync(join(root, path), "utf8");

// The readers, each a `$JsonDecoder`, by the file that writes it.
const readers: Record<string, string> = {
  $JsonReader: "src/runtime/json_reader.js",
  $JsonValueReader: "src/runtime/json_value_reader.js",
  $JsonContent: "src/runtime/json_content.js",
};

// A class's body, from `class Name ` to its closing brace at the margin.
const body = (source: string, name: string) => {
  const start = source.indexOf(`class ${name} `);
  if (start < 0) throw new Error(`no class ${name}`);
  return source.slice(start, source.indexOf("\n}", start));
};
// Its own methods, each `name(` at the body's first indent.
const methods = (text: string) =>
  new Set([...text.matchAll(/^ {2}(?:static )?(?:get )?([A-Za-z_$][\w$]*)\(/gm)].map((m) => m[1]).filter((m) => m !== "constructor"));
// What `text` calls of `receiver`, `receiver.name(`.
const calls = (text: string, receiver: string) =>
  [...text.matchAll(new RegExp(`(?<![\\w$.])${receiver}\\.([A-Za-z_$][\\w$]*)\\(`, "g"))].map((m) => m[1]);

// Each method the code over a reader calls of it, where: `$JsonDecoder`'s
// own, of `this`; a hand-written visitor's hints, `json_visit.js`'s of
// `reader`; and the codecs, `$json`'s, of `json`. Each reader writes it, or
// has it of `$JsonDecoder`; one only some write is no method of the
// interface, which a reader of another backend would throw on, as
// `$jsonVisitOption`'s `reader.parseWhitespace()` did of a `Value` (#1597).
function unmet(source: (path: string) => string): string[] {
  const decoderSource = source("src/runtime/json_decoder.js");
  const decoder = body(decoderSource, "$JsonDecoder");
  const codecs = source("src/runtime/from_json.js");
  const table = codecs.slice(codecs.indexOf("const $json = {"), codecs.indexOf("\n};", codecs.indexOf("const $json = {")));
  const called = new Map<string, string>();
  for (const m of calls(decoder, "this")) called.set(m, "`$JsonDecoder`");
  for (const m of calls(source("src/runtime/json_visit.js"), "reader")) called.set(m, "`json_visit.js`");
  for (const m of calls(table, "json")) called.set(m, "`$json`");
  const inherited = methods(decoder);
  const found: string[] = [];
  for (const [name, file] of Object.entries(readers)) {
    const own = methods(body(source(file), name));
    for (const [m, where] of called) {
      if (!own.has(m) && !inherited.has(m)) found.push(`${name} has no \`${m}\`, which ${where} calls of every reader`);
    }
  }
  return found;
}

test("every method the code over a reader calls is each reader's", () => {
  expect(unmet(read)).toEqual([]);
  // A text reader's own method called of any reader is caught.
  const planted = (path: string) =>
    path === "src/runtime/json_visit.js" ? `${read(path)}\nfunction planted(reader) {\n  return reader.parseWhitespace();\n}\n` : read(path);
  expect(unmet(planted)).toEqual([
    "$JsonValueReader has no `parseWhitespace`, which `json_visit.js` calls of every reader",
    "$JsonContent has no `parseWhitespace`, which `json_visit.js` calls of every reader",
  ]);
});

// Each reader the lowering names, `reader("vec")` of `$json`, and each
// number's, its primitive's name, `$json.u32`, is one `$json` has: a name it
// hasn't would be an `undefined` reader, which throws `read is not a
// function` where the program runs, as a `NonZero<u32>`'s did by its type's
// name (#1538). A type it has none for is refused.
test("each reader the lowering names is one $json has", () => {
  const codecs = read("src/runtime/from_json.js");
  const start = codecs.indexOf("const $json = {");
  const table = codecs.slice(start, codecs.indexOf("\n};", start));
  const has = new Set([...table.matchAll(/^ {2}([A-Za-z_$][\w$]*):/gm)].map((m) => m[1]));
  const lowering = read("src/lower/serde/de.rs");
  const named = [...lowering.matchAll(/\breader\("([A-Za-z_$][\w$]*)"\)/g)].map((m) => m[1]);
  const numbers = ["u8", "u16", "u32", "u64", "usize", "i8", "i16", "i32", "i64", "isize", "f32", "f64"];
  expect(named.length).toBeGreaterThan(10);
  expect([...named, ...numbers].filter((name) => !has.has(name))).toEqual([]);
  // A name made otherwise, of a type's, is a primitive number's alone.
  const made = [...lowering.matchAll(/\breader\(&([^)]*)\)/g)].map((m) => m[1]);
  expect(made).toEqual(["ty.to_string("]);
  expect(lowering).toContain("if Num::of(ty).is_some() && ty.is_primitive() {\n            return Ok(reader(&ty.to_string()));");
});

