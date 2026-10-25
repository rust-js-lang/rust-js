import { expect, test } from "bun:test";
import { mkdtempSync, realpathSync, writeFileSync, rmSync, readFileSync, mkdirSync, existsSync } from "node:fs";
import { tmpdir } from "node:os";
import { join } from "node:path";
import { parseManifest, mapManifestPaths, parseCompilerIdentity } from "../tooling/manifest.js";
import { createNativeBuilder } from "../tooling/build.js";
import { buildCompiler, cargoApp, compiler, fixture, installRuntime, run } from "./support";

const manifest = {
  version: 1, input: "/virtual/lib.rs", output: "/virtual/lib.js",
  sources: ["/virtual/lib.rs"],
  modules: [{ module: [], file: "/virtual/lib.js", map: "/virtual/lib.js.map", source: "/virtual/lib.rs", imports: [] }],
  artifacts: [{ file: "/virtual/lib.js", hash: "1234567890abcdef" }, { file: "/virtual/lib.js.map", hash: "1234567890abcdef" }],
};

test("hosts reject incompatible and malformed manifests before consuming paths", () => {
  for (const value of [null, { ...manifest, version: 2 }, { ...manifest, sources: [123] },
    { ...manifest, output: "relative.js" }, { ...manifest, artifacts: [] },
    { ...manifest, modules: [{ ...manifest.modules[0], imports: ["/missing.js"] }] },
    { ...manifest, modules: [{ ...manifest.modules[0], exports: [1] }] },
    { ...manifest, tests: "/virtual/missing.test.js" }]) {
    expect(() => parseManifest(JSON.stringify(value))).toThrow();
  }
  expect(parseManifest(JSON.stringify(manifest))).toEqual(manifest);
});

// What a module exports, and the runner of a build's `#[test]` functions,
// a host reads of the manifest, not of the JS: the playground ran a
// program's `main` by a regular expression over its JS, and named its
// tests' runner by its own rule (the third architecture audit).
test("a manifest says what each module exports, and where its tests' runner is", () => {
  buildCompiler();
  const dir = fixture("manifest-exports");
  writeFileSync(join(dir, "lib.rs"), `mod util {
    pub fn helper() -> u32 { 1 }
}
pub use util::helper;
pub use self::main as start;
pub const LIMIT: u32 = 3;
// A field, whose JS name a test build, which rustc hashes nothing of, asks.
pub struct Total { pub first_part: u32 }
pub async fn main() {}
fn private() {}
#[test]
fn adds() { assert_eq!(Total { first_part: util::helper() + 1 }.first_part, 2); }
`);
  const out = join(dir, "out");
  run([compiler, "--test", join(dir, "lib.rs"), "-o", join(out, "lib.js"), "--manifest", join(out, "manifest.json")]);
  const read = parseManifest(readFileSync(join(out, "manifest.json"), "utf8"));
  const root = read.modules.find((module: { module: string[] }) => module.module.length === 0);
  // And, built to run them, its `#[test]` functions, which the runner imports.
  // `main` by `start` alone, as `pub use self::main as start;` exports it (ADR 0596, amended).
  expect([...root.exports].sort()).toEqual(["LIMIT", "adds", "helper", "start"]);
  expect(read.tests).toBe(join(realpathSync(out), "lib.test.js"));
  expect(existsSync(read.tests)).toBe(true);
});

test("compiler identities reject incompatible ABI and missing version fields", () => {
  const identity = { version: "0.1.0", toolchain: "1.99.0", abi: 1 };
  expect(parseCompilerIdentity(JSON.stringify(identity))).toEqual(identity);
  for (const value of [null, {}, [], { ...identity, abi: 2 }, { ...identity, version: "" }, { ...identity, toolchain: "" }]) {
    expect(() => parseCompilerIdentity(JSON.stringify(value))).toThrow();
    expect(() => parseManifest(JSON.stringify({ ...manifest, compiler: value }))).toThrow();
  }
});

// A library's contract (ADR 0100): each item another crate can reach, by
// rustc's key for it, with its JS name and the drops it's given; and the
// types whose `Rc`s it counts (ADR 0852).
test("library manifests validate their items and remap fingerprinted inputs", () => {
  const library = {
    version: 7, name: "shared", crate_hash: "0123456789abcdef", inputs: [{ file: "/virtual/shared.rs", hash: "1234567890abcdef" }],
    items: [{ key: "ab12", rust_path: "shared::User::validate", module: [], export: "User", member: "validate" },
      { key: "cd34", rust_path: "shared::count", module: ["util"], export: "count", member: null, params: [{ index: 0, marks: ["drop"] }, { index: 1, stepped: ["9f0e"] }], facts: ["takes_location"] }],
    impls: ["ef56"], libraries: ["base"], counted_rcs: [{ of: "ab12", args: [{}] }],
    ref_cells: { counted: [{ of: "ab12", args: [{}] }], unchecked: [{}] },
    fields: [{ key: "gh78", facts: ["quiet"] }],
  };
  const value = { ...manifest, library };
  expect(parseManifest(JSON.stringify(value))).toEqual(value);
  const mapped = mapManifestPaths(value, (path: string) => path.replace("/virtual", "/local"));
  expect(mapped.library.inputs[0].file).toBe("/local/shared.rs");
  expect(mapped.library.items).toEqual(library.items);
  const item = library.items[0];
  for (const invalid of [null, { ...library, version: 2 }, { ...library, crate_hash: "" }, { ...library, inputs: [{ file: "relative.rs", hash: "bad" }] },
    { ...library, impls: [1] }, { ...library, libraries: "base" }, { ...library, items: [{ ...item, member: 5 }] }, { ...library, items: [{ ...item, params: [{ index: "T" }] }] },
    { ...library, items: [{ ...item, module: "root" }] }, { ...library, items: [{ ...item, params: [{ index: 1, stepped: [0] }] }] }, { ...library, items: [{ ...item, params: [{ index: 0, marks: ["dropped"] }] }] }, { ...library, items: [{ ...item, facts: ["throws"] }] },
    { ...library, counted_rcs: true }, { ...library, counted_rcs: [{ of: 1 }] }, { ...library, counted_rcs: [{ args: [null] }] },
    { ...library, ref_cells: [] }, { ...library, ref_cells: { handed: {} } }, { ...library, ref_cells: { boxed: [{ of: 1 }] } },
    { ...library, fields: {} }, { ...library, fields: [{ key: 1 }] }, { ...library, fields: [{ key: "gh78", facts: ["loud"] }] }]) {
    expect(() => parseManifest(JSON.stringify({ ...manifest, library: invalid }))).toThrow("library contract");
  }
});

test("virtual path mapping preserves JSON escaping and unrelated values", () => {
  const directory = '/local/a"quoted\\folder';
  const value = { ...manifest, note: "/virtual/not-a-path-field" };
  const mapped = mapManifestPaths(parseManifest(JSON.stringify(value)), (path: string) => path.replace("/virtual", directory));
  const reread = parseManifest(JSON.stringify(mapped));
  expect(reread.input).toBe(`${directory}/lib.rs`);
  expect(reread.modules[0].map).toBe(`${directory}/lib.js.map`);
  expect(reread.artifacts[0].hash).toBe(manifest.artifacts[0].hash);
  expect(reread.note).toBe(value.note);
});

// An app's Rust is built with Cargo, its one way (ADR 0101): the compiler's
// identity in what it wrote, and a build that fails leaves the last one's JS.
test("the builder builds an app's Cargo package, and a failed build leaves its JS", async () => {
  buildCompiler();
  const root = realpathSync(mkdtempSync(join(tmpdir(), "rust-js-independent-")));
  installRuntime(root);
  try {
    const options = cargoApp(root, "pub fn answer() -> u32 {\n    42\n}\n");
    const builder = createNativeBuilder({ root, rustJs: compiler });
    const built = await builder.checkCargo(options);
    const result = built.manifests.get("app");
    const version = Bun.spawnSync([compiler, "--version-json"], { stdout: "pipe", stderr: "pipe" });
    expect(version.exitCode).toBe(0);
    expect(parseCompilerIdentity(version.stdout.toString())).toEqual(result.compiler);
    expect(result.sources).toContain(join(root, "src", "lib.rs"));
    expect((await import(built.js)).answer()).toBe(42);
    const previous = readFileSync(built.js, "utf8");
    writeFileSync(join(root, "src", "lib.rs"), "pub fn broken(");
    await expect(builder.checkCargo(options)).rejects.toThrow();
    expect(readFileSync(built.js, "utf8")).toBe(previous);
  } finally { rmSync(root, { recursive: true, force: true }); }
}, 600_000);

// The JS a compiler writes imports helpers of its own release (ADR 0103):
// an app without @rust-js/runtime, or with another version's, is refused
// before anything's compiled.
test("the builder refuses an app without the compiler's @rust-js/runtime", async () => {
  buildCompiler();
  const root = realpathSync(mkdtempSync(join(tmpdir(), "rust-js-runtime-version-")));
  try {
    const options = cargoApp(root, "pub fn answer() -> u32 {\n    42\n}\n");
    const builder = () => createNativeBuilder({ root, rustJs: compiler });
    const version = parseCompilerIdentity(Bun.spawnSync([compiler, "--version-json"], { stdout: "pipe" }).stdout.toString()).version;
    await expect(builder().checkCargo(options)).rejects.toThrow(`imports @rust-js/runtime ${version}: install it`);
    mkdirSync(join(root, "node_modules", "@rust-js", "runtime"), { recursive: true });
    writeFileSync(join(root, "node_modules", "@rust-js", "runtime", "package.json"), '{ "name": "@rust-js/runtime", "version": "999.0.0", "exports": { "./package.json": "./package.json" } }\n');
    await expect(builder().checkCargo(options)).rejects.toThrow(`needs @rust-js/runtime ${version}, not the installed 999.0.0`);
    rmSync(join(root, "node_modules"), { recursive: true });
    installRuntime(root);
    const built = await builder().checkCargo(options);
    expect((await import(built.js)).answer()).toBe(42);
  } finally { rmSync(root, { recursive: true, force: true }); }
}, 600_000);

// An app's Cargo builds run the Rust the compiler says it was built with,
// its `--version-json`.
test("the builder's Cargo builds run the compiler's Rust", async () => {
  buildCompiler();
  const root = realpathSync(mkdtempSync(join(tmpdir(), "rust-js-cargo-app-")));
  try {
    const options = cargoApp(root, "pub fn answer() -> i32 {\n    42\n}\n");
    installRuntime(root);
    const builder = createNativeBuilder({ root, rustJs: compiler });
    const { packages } = await builder.cargoWorkspace({ manifestPath: options.manifestPath, offline: true });
    expect([...packages.keys()]).toEqual(["app"]);
    const built = await builder.checkCargo(options);
    expect(readFileSync(built.js, "utf8")).toContain("export function answer() {\n  return 42;\n}");
  } finally { rmSync(root, { recursive: true, force: true }); }
}, 600_000);

// ADR 0101: a module's `links` are where its JS and its declarations name
// another module's file, by the compiler's parse of what it printed: each
// relative specifier's bytes, after a directive and in an `export .. from`
// too, and none of a package's, `@rust-js/runtime`.
test("a manifest's links are each relative specifier a module's files print", async () => {
  buildCompiler();
  const dir = fixture("manifest-links");
  writeFileSync(join(dir, "Cargo.toml"), '[package]\nname = "app"\nversion = "0.0.0"\nedition = "2024"\n');
  writeFileSync(join(dir, "lib.rs"), `pub struct Value {
    pub n: u32,
}
pub fn base(n: Option<u32>) -> u32 {
    n.unwrap()
}
pub mod child {
    #[cfg_attr(rust_js, rust_js::directive = "use client")]
    const _: () = ();
    pub use crate::base;
    pub fn make() -> crate::Value {
        crate::Value { n: crate::base(Some(1)) }
    }
}
pub mod deep {
    pub mod inner {
        pub fn two() -> u32 {
            crate::base(Some(2))
        }
    }
}
`);
  const manifestPath = join(dir, "manifest.json");
  const p = Bun.spawnSync([compiler, "--declarations", join(dir, "lib.rs"), "-o", join(dir, "lib.js"), "--manifest", manifestPath], { stderr: "pipe" });
  expect(p.exitCode, p.stderr.toString()).toBe(0);
  const manifest = parseManifest(readFileSync(manifestPath, "utf8"));
  const named = (file: string, links: { start: number; end: number; file: string }[]) =>
    links.map((link) => [readFileSync(file).subarray(link.start, link.end).toString(), link.file]);
  const lib = join(realpathSync(dir), "lib.js");
  const root = manifest.modules.find((m: { module: string[] }) => m.module.length === 0);
  const child = manifest.modules.find((m: { module: string[] }) => m.module[0] === "child");
  expect(named(root.file, root.links)).toEqual([]);
  expect(readFileSync(root.file, "utf8")).toContain('from "@rust-js/runtime"');
  expect(named(child.file, child.links)).toEqual([["./lib.js", lib], ["./lib.js", lib]]);
  expect(named(child.types, child.type_links)).toEqual([["./lib.js", lib], ["./lib.js", lib]]);
  const inner = manifest.modules.find((m: { module: string[] }) => m.module.join("::") === "deep::inner");
  expect(named(inner.file, inner.links)).toEqual([["../lib.js", lib]]);
});

// A library's JS is found by its manifest (ADR 0042): one asked for without
// a manifest is refused, before anything is written.
test("a library is refused without its manifest", () => {
  buildCompiler();
  const dir = fixture("library-without-manifest");
  writeFileSync(join(dir, "lib.rs"), "pub fn one() -> u32 { 1 }\n");
  expect(() => run([compiler, join(dir, "lib.rs"), "--library"])).toThrow("--library requires --manifest");
  expect(existsSync(join(dir, "lib.js"))).toBe(false);
});
