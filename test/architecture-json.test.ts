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
