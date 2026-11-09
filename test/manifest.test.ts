import { expect, test } from "bun:test";
import { mkdtempSync, realpathSync, writeFileSync, rmSync, readFileSync, mkdirSync, existsSync, statSync } from "node:fs";
import { tmpdir } from "node:os";
import { join } from "node:path";
import { LIBRARY_ABI, parseManifest, mapManifestPaths, parseCompilerIdentity } from "../tooling/manifest.js";
import { createNativeBuilder } from "../tooling/build.js";
import { buildCompiler, cargoApp, compiler, fixture, installStd, KeptApp, root as repository, run, stdDir } from "./support";

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
    // ADR 1076: the one entry of an older build's tests, and test files it didn't write.
    { ...manifest, tests: "/virtual/lib.test.js" },
    { ...manifest, tests: { runner: "node", module: "node:test", files: ["/virtual/missing.test.js"] } }]) {
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
  // Its test files, and the runner they're for (ADR 1076).
  expect(read.tests).toEqual({ runner: "node", module: "node:test", files: [join(realpathSync(out), "lib.test.js")] });
  expect(existsSync(read.tests.files[0])).toBe(true);
});

test("compiler identities reject incompatible ABI and missing version fields", () => {
  const identity = { version: "0.1.0", toolchain: "1.99.0", abi: 1 };
  expect(parseCompilerIdentity(JSON.stringify(identity))).toEqual(identity);
  // ADR 1003: and which build of it, a hash of what it's made of.
  const built = { ...identity, build: "52c100506ffb5402" };
  expect(parseCompilerIdentity(JSON.stringify(built))).toEqual(built);
  for (const value of [null, {}, [], { ...identity, abi: 2 }, { ...identity, version: "" }, { ...identity, toolchain: "" }, { ...identity, build: "" }, { ...identity, build: 1 }]) {
    expect(() => parseCompilerIdentity(JSON.stringify(value))).toThrow();
    expect(() => parseManifest(JSON.stringify({ ...manifest, compiler: value }))).toThrow();
  }
});

// A library's contract (ADR 0100): each item another crate can reach, by
// rustc's key for it, with its JS name and the drops it's given; and the
// types whose `Rc`s it counts (ADR 0852).
test("library manifests validate their items and remap fingerprinted inputs", () => {
  const library = {
    version: LIBRARY_ABI, name: "shared", crate_hash: "0123456789abcdef", inputs: [{ file: "/virtual/shared.rs", hash: "1234567890abcdef" }],
    items: [{ key: "ab12", rust_path: "shared::User::validate", module: [], export: "User", member: "validate" },
      { key: "cd34", rust_path: "shared::count", module: ["util"], export: "count", member: null, params: [{ index: 0, marks: ["drop"] }, { index: 1, marks: ["copy", "clone"], stepped: ["9f0e"] }, { index: 3, facts: ["Size", "Align"], built: { of: { adt: { key: "0f1e", args: [{ slot: 2 }, { tuple: [{ primitive: "u8" }, { ref: { mutable: false, to: { array: { len: 4, of: { slot: 0 } } } } }] }] } }, gates: [0] } }], facts: ["takes_location"], nulled_params: [0] }],
    impls: ["ef56"], libraries: ["base"], counted_rcs: [{ of: "ab12", args: [{}] }],
    ref_cells: { counted: [{ of: "ab12", args: [{}] }], unchecked: [{}] },
    fields: [{ key: "gh78", facts: ["quiet"] }, { key: "ij90", facts: ["rust_functions"] }],
    built: ["kl12"], associated: [{ key: "mn34", facts: ["Size"] }],
  };
  const value = { ...manifest, library };
  expect(parseManifest(JSON.stringify(value))).toEqual(value);
  const mapped = mapManifestPaths(value, (path: string) => path.replace("/virtual", "/local"));
  expect(mapped.library.inputs[0].file).toBe("/local/shared.rs");
  expect(mapped.library.items).toEqual(library.items);
  const item = library.items[0];
  for (const invalid of [null, { ...library, version: 2 }, { ...library, crate_hash: "" }, { ...library, inputs: [{ file: "relative.rs", hash: "bad" }] },
    { ...library, impls: [1] }, { ...library, libraries: "base" }, { ...library, items: [{ ...item, member: 5 }] }, { ...library, items: [{ ...item, params: [{ index: "T" }] }] },
    { ...library, items: [{ ...item, module: "root" }] }, { ...library, items: [{ ...item, params: [{ index: 1, stepped: [0] }] }] }, { ...library, items: [{ ...item, params: [{ index: 0, marks: ["dropped"] }] }] }, { ...library, items: [{ ...item, params: [{ index: 3, built: { of: { slot: 0, tuple: [] } } }] }] }, { ...library, items: [{ ...item, params: [{ index: 3, built: { of: { slot: 0 }, gates: ["0"] } }] }] }, { ...library, items: [{ ...item, facts: ["throws"] }] },
    { ...library, counted_rcs: true }, { ...library, counted_rcs: [{ of: 1 }] }, { ...library, counted_rcs: [{ args: [null] }] },
    { ...library, ref_cells: [] }, { ...library, ref_cells: { handed: {} } }, { ...library, ref_cells: { boxed: [{ of: 1 }] } },
    { ...library, fields: {} }, { ...library, fields: [{ key: 1 }] }, { ...library, fields: [{ key: "gh78", facts: ["loud"] }] }, { ...library, built: "kl12" }, { ...library, built: [1] },
    { ...library, items: [{ ...item, nulled_params: ["0"] }] }, { ...library, items: [{ ...item, nulled_params: 0 }] },
    { ...library, associated: {} }, { ...library, associated: [{ key: "mn34", facts: "Size" }] }]) {
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
  installStd(root);
  try {
    const options = cargoApp(root, "pub fn answer() -> u32 {\n    42\n}\n");
    const builder = createNativeBuilder({ root, rustJs: compiler });
    const built = await builder.checkCargo(options);
    const result = built.manifests.get("app");
    const version = Bun.spawnSync([compiler, "--version-json"], { stdout: "pipe", stderr: "pipe" });
    expect(version.exitCode).toBe(0);
    expect(parseCompilerIdentity(version.stdout.toString())).toEqual(result.compiler);
    expect(result.sources).toContain(join(root, "src", "lib.rs"));
    expect((await import(built.js!)).answer()).toBe(42);
    const previous = readFileSync(built.js!, "utf8");
    writeFileSync(join(root, "src", "lib.rs"), "pub fn broken(");
    await expect(builder.checkCargo(options)).rejects.toThrow();
    expect(readFileSync(built.js!, "utf8")).toBe(previous);
  } finally { rmSync(root, { recursive: true, force: true }); }
}, 600_000);

// The JS a compiler writes imports helpers of its own release (ADR 0103):
// an app without @rust-js/std, or with another version's, is refused
// before anything's compiled.
test("the builder refuses an app without the compiler's @rust-js/std", async () => {
  buildCompiler();
  const root = realpathSync(mkdtempSync(join(tmpdir(), "rust-js-runtime-version-")));
  try {
    const options = cargoApp(root, "pub fn answer() -> u32 {\n    42\n}\n");
    const builder = () => createNativeBuilder({ root, rustJs: compiler });
    const version = parseCompilerIdentity(Bun.spawnSync([compiler, "--version-json"], { stdout: "pipe" }).stdout.toString()).version;
    await expect(builder().checkCargo(options)).rejects.toThrow(`imports @rust-js/std ${version}: install it`);
    mkdirSync(join(root, "node_modules", "@rust-js", "std"), { recursive: true });
    writeFileSync(join(root, "node_modules", "@rust-js", "std", "package.json"), '{ "name": "@rust-js/std", "version": "999.0.0", "exports": { "./package.json": "./package.json" } }\n');
    await expect(builder().checkCargo(options)).rejects.toThrow(`needs @rust-js/std ${version}, not the installed 999.0.0`);
    rmSync(join(root, "node_modules"), { recursive: true });
    installStd(root);
    const built = await builder().checkCargo(options);
    expect((await import(built.js!)).answer()).toBe(42);
  } finally { rmSync(root, { recursive: true, force: true }); }
}, 600_000);

// An app's Cargo builds run the Rust the compiler says it was built with,
// its `--version-json`.
test("the builder's Cargo builds run the compiler's Rust", async () => {
  buildCompiler();
  const root = realpathSync(mkdtempSync(join(tmpdir(), "rust-js-cargo-app-")));
  try {
    const options = cargoApp(root, "pub fn answer() -> i32 {\n    42\n}\n");
    installStd(root);
    const builder = createNativeBuilder({ root, rustJs: compiler });
    const { packages } = await builder.cargoWorkspace({ manifestPath: options.manifestPath, offline: true });
    expect([...packages.keys()]).toEqual(["app"]);
    const built = await builder.checkCargo(options);
    expect(readFileSync(built.js!, "utf8")).toContain("export function answer() {\n  return 42;\n}");
  } finally { rmSync(root, { recursive: true, force: true }); }
}, 600_000);

// ADR 0101: a module's `links` are where its JS and its declarations name
// another module's file, by the compiler's parse of what it printed: each
// relative specifier's bytes, after a directive and in an `export .. from`
// too, one of what the module doesn't import, `other` (rd705), and none of
// a package's, `@rust-js/std`.
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
pub fn other() -> u32 {
    3
}
pub mod child {
    #[cfg_attr(rust_js, rust_js::directive = "use client")]
    const _: () = ();
    pub use crate::{base, other};
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
  expect(readFileSync(root.file, "utf8")).toContain('from "@rust-js/std"');
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

// The library ABI the tooling reads, `LIBRARY_ABI` in tooling/manifest.js,
// mirrors the compiler's, `src/library.rs`' `VERSION`: the std package's
// manifest, which the compiler writes and every compile checks is of its
// `VERSION`, says the ABI the tooling takes.
test("the tooling's library ABI is the compiler's", () => {
  const std = JSON.parse(readFileSync(join(import.meta.dir, "..", "std", "manifest.json"), "utf8"));
  expect(std.library.version).toBe(LIBRARY_ABI);
});

// An app kept between runs (`KeptApp`, #882) is built again where anything
// it's built from changed, and found built where nothing did: as Cargo
// judges it, by its fingerprints. What a run before wrote that this one
// doesn't is gone, and so is a `Cargo.lock` it didn't write; two claims of
// one name at once are two directories; a file changed mid-test, then
// changed back by the next run's write, is built again each time; and a
// crate it depends on by path, a binding crate's source say, changed, is
// built again with the app.
test("an app kept between runs is built again where what it's built from changed, and only then", async () => {
  buildCompiler();
  const builder = createNativeBuilder({ root: repository, rustJs: compiler });
  const binding = fixture("kept-app-binding");
  writeFileSync(join(binding, "Cargo.toml"), '[package]\nname = "binding"\nversion = "0.0.0"\nedition = "2024"\n\n[lib]\npath = "lib.rs"\n');
  writeFileSync(join(binding, "lib.rs"), "pub fn base() -> u32 {\n    40\n}\n");
  const lib = "mod extra;\npub fn answer() -> u32 {\n    binding::base() + extra::two()\n}\n";
  const write = (app: KeptApp, text = lib) => {
    app.write("Cargo.toml", `[package]\nname = "app"\nversion = "0.0.0"\nedition = "2024"\n\n[dependencies]\nbinding = { path = ${JSON.stringify(binding)} }\n\n[workspace]\n`);
    app.write("src/lib.rs", text);
    app.write("src/extra.rs", "pub fn two() -> u32 {\n    2\n}\n");
    installStdIn(app);
    return { manifestPath: join(app.done(), "Cargo.toml"), packageNames: ["app"], offline: true };
  };
  const answer = async (options: { manifestPath: string; packageNames: string[]; offline: boolean }) => {
    const built = await builder.checkCargo(options);
    // In a process of its own: an import here keeps each module it reached.
    return Number(run([process.execPath, "-e", `import { answer } from ${JSON.stringify(built.js!)}; console.log(answer());`]).trim());
  };
  const first = new KeptApp("kept-app");
  // Another claim of the name, at once, is another directory.
  const other = new KeptApp("kept-app");
  expect(other.dir).not.toBe(first.dir);
  other.release();
  const options = write(first);
  expect(await answer(options)).toBe(42);
  // Nothing changed: found built, its JS as it was.
  const js = (await builder.checkCargo(options)).js!;
  const made = statSync(js).mtimeMs;
  // Its files written again, as the next run writes them, each the same.
  expect((await builder.checkCargo(write(first))).js).toBe(js);
  expect(statSync(js).mtimeMs).toBe(made);
  // Changed mid-test, as a test that builds again changes it.
  writeFileSync(join(first.dir, "src", "lib.rs"), lib.replace("+ extra::two()", "+ extra::two() + 1"));
  expect(await answer(options)).toBe(43);
  first.release();
  // The next run: the same directory, its text written back, built again.
  const next = new KeptApp("kept-app");
  expect(next.dir).toBe(first.dir);
  writeFileSync(join(next.dir, "src", "stale.rs"), "this run didn't write it");
  writeFileSync(join(next.dir, "Cargo.lock"), "# not this run's\n");
  expect(await answer(write(next))).toBe(42);
  // The lock it found is gone, and the build's own is there; but `stale.rs`
  // isn't a file a run wrote, so it's the test's own, kept.
  expect(readFileSync(join(next.dir, "Cargo.lock"), "utf8")).not.toContain("not this run's");
  // A crate it depends on by path changed: built again.
  writeFileSync(join(binding, "lib.rs"), "pub fn base() -> u32 {\n    50\n}\n");
  expect(await answer(write(next))).toBe(52);
  next.release();
  // A run that writes no `extra.rs`: what the one before wrote is gone.
  const last = new KeptApp("kept-app");
  last.write("src/lib.rs", "pub fn answer() -> u32 {\n    7\n}\n");
  last.done();
  expect([existsSync(join(last.dir, "src", "extra.rs")), existsSync(join(last.dir, "Cargo.toml"))]).toEqual([false, false]);
  last.release();
}, 600_000);

/** `@rust-js/std` linked into a kept app, as `installStd` installs it. */
function installStdIn(app: KeptApp) {
  app.link("node_modules/@rust-js/std", stdDir);
}
