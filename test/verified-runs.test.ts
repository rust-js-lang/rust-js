// A corpus case's runs are kept by the std code its JS can run, not by all
// of std (#2352, ADR 0567, amended): what one bundle of the JS keeps of std,
// every module's evaluation and what the harness calls of std among it.
import { expect, test } from "bun:test";
import { existsSync, mkdirSync, readdirSync, readFileSync, writeFileSync } from "node:fs";
import { join } from "node:path";

import { fixture } from "./support";
import { reached, shared } from "./verified-runs";

const files = {
  "package.json": `{"name":"@rust-js/std","type":"module","sideEffects":false,"exports":{".":"./index.js"}}`,
  "index.js": `export * from "./a.js";\nexport * from "./b.js";\n`,
  "a.js": `export function used() {\n  return 1;\n}\nexport function unused() {\n  return 2;\n}\nexport function $isPanic(error) {\n  return error === 3;\n}\n`,
  // A module the JS imports nothing of, whose evaluation, which a run's
  // \`import\` of std makes, keeps a function.
  "b.js": `export const registry = new Map();\nexport const kept = registry.set("k", function hidden() {\n  return 4;\n});\n`,
};

/** A std of `files`, each changed as `edit` says, in a folder of its own. */
let stds = 0;
function std(dir: string, edit: (name: string, text: string) => string = (_, text) => text): string {
  const at = join(dir, `std-${stds++}`);
  mkdirSync(at, { recursive: true });
  for (const [name, text] of Object.entries(files)) writeFileSync(join(at, name), edit(name, text));
  return at;
}

function program(dir: string): string {
  const at = join(dir, "program");
  mkdirSync(at, { recursive: true });
  writeFileSync(join(at, "case.js"), `import { used } from "@rust-js/std";\nimport { more } from "./more.js";\nconsole.log(used(), more());\n`);
  writeFileSync(join(at, "more.js"), `export function more() {\n  return 5;\n}\n`);
  return join(at, "case.js");
}

test("a std edit the JS can't run keeps its runs' key; one it can run, a module's evaluation or a helper the harness calls, doesn't", async () => {
  const dir = fixture("std-reached");
  const entry = program(dir);
  const store = join(dir, "store");
  const of = (edit?: (name: string, text: string) => string) => reached(entry, std(dir, edit), store);
  const base = await of();
  // Not run: a function nothing calls, and a comment.
  expect(await of((name, text) => (name === "a.js" ? text.replace("return 2;", "return 20;") : text))).toBe(base);
  expect(await of((name, text) => (name === "a.js" ? text.replace("export function used", "// The one used.\nexport function used") : text))).toBe(base);
  // Run: what the JS calls, what a module's evaluation keeps, though std
  // says it has no side effects, and what the oracle calls.
  expect(await of((name, text) => (name === "a.js" ? text.replace("return 1;", "return 10;") : text))).not.toBe(base);
  expect(await of((name, text) => (name === "b.js" ? text.replace("return 4;", "return 40;") : text))).not.toBe(base);
  expect(await of((name, text) => (name === "a.js" ? text.replace("error === 3", "error === 30") : text))).not.toBe(base);
});

test("a kept reach is read again only while each file of the JS it bundled is as it was", async () => {
  const dir = fixture("std-reached-kept");
  const entry = program(dir);
  const store = join(dir, "store");
  const given = std(dir);
  const first = await reached(entry, given, store);
  expect(readdirSync(store).filter((id) => existsSync(join(store, id, "reach.json"))).length).toBe(1);
  expect(await reached(entry, given, store)).toBe(first);
  // The JS's other file changed, its entry the same: bundled again.
  writeFileSync(join(entry, "../more.js"), `export function more() {\n  return 50;\n}\n`);
  const again = await reached(entry, given, store);
  expect(again).not.toBe(first);
  expect(again).toBe(await reached(entry, given, join(dir, "fresh")));
});

// The bundler keeps what code reaches by a name, a dictionary's or a
// vtable's object whole; std reaches none but so.
test("std has no code reached but by a name: no eval, no Function, no import()", () => {
  const stdDir = join(import.meta.dir, "..", "std");
  for (const file of readdirSync(stdDir).filter((name) => name.endsWith(".js"))) {
    const text = readFileSync(join(stdDir, file), "utf8");
    expect([file, /\beval\s*\(|\bnew\s+Function\s*\(|\bFunction\s*\(|\bimport\s*\(/.test(text)]).toEqual([file, false]);
  }
});

test("a run's key has the std code its JS reaches, by the JS's text and not its path", async () => {
  const dir = fixture("std-reached-shared");
  const write = (name: string, helper: string) => {
    mkdirSync(join(dir, name), { recursive: true });
    writeFileSync(join(dir, name, "case.js"), `import { ${helper} } from "@rust-js/std";\nconsole.log(${helper});\n`);
    return join(dir, name, "case.js");
  };
  const [index, at, again] = [write("index", "$index"), write("at", "$at"), write("again", "$index")];
  // The checkout's std, which a mutant's target has none of.
  const std = join(import.meta.dir, "..", "std");
  expect(await shared(index, std)).not.toBe(await shared(at, std));
  expect(await shared(again, std)).toBe(await shared(index, std));
});
