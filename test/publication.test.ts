// A build publishes the same way whichever host runs it: the native
// compiler (src/output.rs) and the WASI host's publisher (tooling/publish.js)
// remove a file an older build wrote only if it's still theirs to remove
// (ADR 0091). Each case here runs against both, from the same older manifest.
//
//   older manifest: artifacts [lib.js, X]  ──►  this build writes lib.js
//                                          ──►  X removed, or kept?

import { beforeAll, expect, test } from "bun:test";
import { existsSync, mkdirSync, readFileSync, symlinkSync, writeFileSync } from "node:fs";
import { join } from "node:path";

import { commit, fingerprint, publishArtifacts } from "../tooling/publish.js";
import { buildCompiler, compiler, fixture, run } from "./support";

beforeAll(buildCompiler, 600_000);

type Case = {
  name: string;
  /** The older build's artifact, relative to the case's folder. */
  file: string;
  kept: boolean;
  /** Whether this build reads it, as `include_str!` does. */
  read?: boolean;
  /** A symlink to what this build reads, where the artifact was. */
  alias?: boolean;
  edited?: boolean;
};

const cases: Case[] = [
  { name: "an artifact this build doesn't write", file: "out/old.js", kept: false },
  { name: "an obsolete declaration", file: "out/old.d.ts", kept: false },
  { name: "a declaration this build reads", file: "out/helper.d.ts", read: true, kept: true },
  { name: "a declaration alias this build reads", file: "out/alias.d.ts", read: true, alias: true, kept: true },
  { name: "an edited declaration", file: "out/edited.d.ts", edited: true, kept: true },
  { name: "a declaration outside the output directory", file: "elsewhere/old.d.ts", kept: true },
  { name: "ordinary TypeScript", file: "out/manual.ts", kept: true },
  { name: "an older map", file: "out/old.js.map", kept: false },
  { name: "one this build reads", file: "out/helper.js", read: true, kept: true },
  { name: "a path to what this build reads", file: "out/alias.js", read: true, alias: true, kept: true },
  { name: "one outside the output's directory", file: "elsewhere/old.js", kept: true },
  { name: "one that isn't JS or a map", file: "out/notes.txt", kept: true },
  { name: "one changed since it was written", file: "out/edited.js", edited: true, kept: true },
];

/** The folder for a case, its input and output, and the older manifest. */
function setUp(c: Case, host: string) {
  const dir = fixture(`publication-${host}`);
  mkdirSync(join(dir, "out"), { recursive: true });
  mkdirSync(join(dir, "elsewhere"), { recursive: true });
  const input = join(dir, "lib.rs");
  const output = join(dir, "out", "lib.js");
  const manifest = join(dir, "out", "lib.manifest.json");
  const file = join(dir, c.file);
  const helperName = c.file.endsWith(".d.ts") ? "helper.d.ts" : "helper.js";
  const helper = join(dir, "out", helperName);
  writeFileSync(helper, "export const helper = 1;\n");
  if (c.alias) symlinkSync(helper, file);
  else if (c.file !== `out/${helperName}`) writeFileSync(file, `written by an older build: ${c.name}\n`);
  const text = readFileSync(file);
  const read = c.read ? `pub const HELPER: &str = include_str!("out/${helperName}");\n` : "";
  writeFileSync(input, `${read}pub fn answer() -> i32 { 42 }\n`);
  const older = {
    version: 1,
    input,
    output,
    sources: [input],
    modules: [],
    artifacts: [{ file, hash: fingerprint(text) }],
  };
  writeFileSync(manifest, JSON.stringify(older, null, 2));
  if (c.edited) writeFileSync(file, "a person's edit\n");
  return { dir, input, output, manifest, file, helper };
}

for (const c of cases) {
  test(`native: ${c.name} is ${c.kept ? "kept" : "removed"}`, () => {
    const { input, output, manifest, file } = setUp(c, "native");
    const p = Bun.spawnSync([compiler, input, "-o", output, "--manifest", manifest], { stderr: "pipe" });
    expect(p.stderr.toString()).toBe("");
    expect(existsSync(file)).toBe(c.kept);
  });

  test(`WASI host: ${c.name} is ${c.kept ? "kept" : "removed"}`, () => {
    const { input, output, manifest, file, helper } = setUp(c, "host");
    const lib = Buffer.from("export function answer() { return 42; }\n");
    const next = {
      version: 1,
      input,
      output,
      sources: c.read ? [input, helper] : [input],
      modules: [],
      artifacts: [{ file: output, hash: fingerprint(lib) }],
    };
    publishArtifacts(manifest, next, new Map([[output, lib]]));
    expect(existsSync(file)).toBe(c.kept);
  });
}

// A file a build replaces is never missing, even for a moment: a bundler
// watching it, Next.js's Turbopack, would take it for deleted, and what
// imports it for broken (ADR 0192). Another process looks for it while it's
// replaced, again and again.
test("a file commit replaces is never missing while it's replaced", async () => {
  const dir = fixture("commit-atomic");
  const file = join(dir, "page.jsx"), stop = join(dir, "stop");
  writeFileSync(file, "0");
  const looking = Bun.spawn([process.execPath, "-e", `const fs = require("fs"); let missing = 0;
console.log("looking");
while (!fs.existsSync(${JSON.stringify(stop)})) if (!fs.existsSync(${JSON.stringify(file)})) missing++;
console.log(missing);`], { stdout: "pipe" });
  const reader = looking.stdout.getReader();
  let said = "";
  while (!said.includes("looking")) said += new TextDecoder().decode((await reader.read()).value);
  for (let i = 1; i <= 2000; i++) commit(new Map([[file, String(i)]]), []);
  writeFileSync(stop, "");
  for (let chunk; !(chunk = await reader.read()).done;) said += new TextDecoder().decode(chunk.value);
  expect([readFileSync(file, "utf8"), Number(said.replace("looking", "").trim())]).toEqual(["2000", 0]);
});

// Turning declarations off must leave the same owned output as a clean build,
// while preserving a declaration someone edited (ADRs 0091, 0196).
test("native: removing modules and disabling declarations cleans up generated declarations", () => {
  const dir = fixture("declarations-history");
  const config = join(dir, "Cargo.toml");
  const input = join(dir, "lib.rs"), output = join(dir, "lib.js"), manifest = join(dir, "manifest.json");
  writeFileSync(config, '[package]\nname = "app"\nversion = "0.0.0"\nedition = "2024"\n[package.metadata.rust-js]\ndeclarations = true\n');
  writeFileSync(input, 'pub mod old { pub fn removed() -> u32 { 7 } } pub mod edited { pub fn value() -> u32 { 8 } } pub fn ready() -> u32 { 1 }');
  run([compiler, input, "-o", output, "--manifest", manifest]);
  const rootTypes = join(dir, "lib.d.ts"), oldTypes = join(dir, "old.d.ts"), editedTypes = join(dir, "edited.d.ts");
  for (const file of [rootTypes, oldTypes, editedTypes]) expect(existsSync(file)).toBe(true);
  writeFileSync(input, 'pub mod edited { pub fn value() -> u32 { 8 } } pub fn ready() -> u32 { 1 }');
  run([compiler, input, "-o", output, "--manifest", manifest]);
  expect(existsSync(rootTypes)).toBe(true);
  expect(existsSync(oldTypes)).toBe(false);
  expect(existsSync(editedTypes)).toBe(true);
  writeFileSync(editedTypes, "// user edit\n");
  writeFileSync(config, readFileSync(config, "utf8").replace("declarations = true", "declarations = false"));
  writeFileSync(input, 'pub fn ready() -> u32 { 1 }');
  run([compiler, input, "-o", output, "--manifest", manifest]);
  expect(existsSync(rootTypes)).toBe(false);
  expect(existsSync(oldTypes)).toBe(false);
  expect(readFileSync(editedTypes, "utf8")).toBe("// user edit\n");
  const result = JSON.parse(readFileSync(manifest, "utf8"));
  expect(result.artifacts.some((artifact: { file: string }) => artifact.file.endsWith(".d.ts"))).toBe(false);
});
