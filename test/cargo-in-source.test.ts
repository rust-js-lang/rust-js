// A Cargo build's JS in source (ADR 0101): each module's beside its Rust,
// as ReScript writes it and a project commits it (ADR 0041).

import { beforeAll, expect, test } from "bun:test";
import { existsSync, mkdirSync, readdirSync, readFileSync, rmSync, writeFileSync } from "node:fs";
import { basename, join } from "node:path";
import { checkCargo } from "../tooling/cargo.js";
import { node } from "./programs";
import { buildCompiler, compiler, fixture, root, run } from "./support";
import { pin } from "./crates";

beforeAll(buildCompiler, 600_000);

// An inline module's JS is where its file would be, `mod inner { .. }` of
// `src/lib.rs` beside it as `src/inner.js`, and one in `src/api.rs` in
// `src/api/`: in the parent's file, one would overwrite the other.
test("a Cargo build's JS in source has each inline module where its file would be", async () => {
  const dir = fixture("cargo-in-source-inline");
  const src = join(dir, "src");
  mkdirSync(src, { recursive: true });
  writeFileSync(join(dir, "Cargo.toml"), '[package]\nname = "app"\nversion = "0.1.0"\nedition = "2024"\n');
  writeFileSync(join(src, "lib.rs"), "mod api;\n\npub mod inner {\n    pub fn value() -> u32 {\n        42\n    }\n}\n\npub fn answer() -> u32 {\n    inner::value() + api::nested::one()\n}\n");
  writeFileSync(join(src, "api.rs"), "pub mod nested {\n    pub fn one() -> u32 {\n        1\n    }\n}\n");
  const { js, files } = await checkCargo({ manifestPath: join(dir, "Cargo.toml"), toolchain: pin, compiler, offline: true, packageName: "app", inSource: true });
  expect(js).toBe(join(src, "lib.js"));
  // `api` has nothing of its own, so no JS: `nested` is still in `src/api/`.
  expect([...files].sort()).toEqual([join(src, "api", "nested.js"), join(src, "inner.js"), join(src, "lib.js")]);
  expect(run([node ?? "node", "--input-type=module", "--eval", `console.log((await import(${JSON.stringify(js)})).answer());`]).trim()).toBe("43");
  for (const file of files) expect(readFileSync(file, "utf8")).toContain(`//# sourceMappingURL=${basename(file)}.map`);
}, 600_000);

// A root of another name, the template's `[lib] path = "src/App.rs"`, is
// `App.js` beside it, which names its own map, `App.js.map`, not the name
// rust-js wrote it under, the build's `lib.js`: else the page's source maps
// aren't found.
test("a Cargo build's JS in source names its own map, for a root of another name", async () => {
  const dir = fixture("cargo-in-source-root");
  const src = join(dir, "src");
  mkdirSync(src, { recursive: true });
  writeFileSync(join(dir, "Cargo.toml"), '[package]\nname = "app"\nversion = "0.1.0"\nedition = "2024"\n\n[lib]\npath = "src/App.rs"\n');
  writeFileSync(join(src, "App.rs"), "pub fn answer() -> u32 {\n    42\n}\n");
  const { js } = await checkCargo({ manifestPath: join(dir, "Cargo.toml"), toolchain: pin, compiler, offline: true, packageName: "app", inSource: true });
  expect(js).toBe(join(src, "App.js"));
  expect(readFileSync(js, "utf8")).toEndWith("//# sourceMappingURL=App.js.map\n");
  expect(JSON.parse(readFileSync(`${js}.map`, "utf8")).file).toBe("App.js");
}, 600_000);

// A module where its file would be can be where the crate's root is:
// `mod root { .. }` of a `[lib] path = "src/root.rs"`. Neither is written
// over the other: it's an error, as rust-js's own `mod lib` of `lib.rs` is.
test("a Cargo build's JS in source refuses two modules for one file", async () => {
  const dir = fixture("cargo-in-source-collision");
  const src = join(dir, "src");
  mkdirSync(src, { recursive: true });
  writeFileSync(join(dir, "Cargo.toml"), '[package]\nname = "app"\nversion = "0.1.0"\nedition = "2024"\n\n[lib]\npath = "src/root.rs"\n');
  writeFileSync(join(src, "root.rs"), "pub mod root {\n    pub fn value() -> u32 {\n        42\n    }\n}\n\npub fn answer() -> u32 {\n    root::value()\n}\n");
  const inSource = () => checkCargo({ manifestPath: join(dir, "Cargo.toml"), toolchain: pin, compiler, offline: true, packageName: "app", inSource: true });
  await expect(inSource()).rejects.toThrow(/the crate root and module `root` would both be .*src\/root\.js/);
  expect(existsSync(join(src, "root.js"))).toBe(false);
}, 600_000);

// Two crates' modules can be where one file would be too: an inline
// `mod helper` of `sources/alpha.rs`'s and of `sources/beta.rs`'s.
test("a Cargo build's JS in source refuses two crates' modules for one file", async () => {
  const dir = fixture("cargo-in-source-crates-collision");
  mkdirSync(join(dir, "sources"), { recursive: true });
  writeFileSync(join(dir, "Cargo.toml"), '[workspace]\nmembers = ["alpha", "beta"]\nresolver = "2"\n');
  for (const [name, dependency] of [["alpha", 'beta = { path = "../beta" }\n'], ["beta", ""]]) {
    mkdirSync(join(dir, name));
    writeFileSync(join(dir, name, "Cargo.toml"), `[package]\nname = "${name}"\nversion = "0.1.0"\nedition = "2024"\n\n[lib]\npath = "../sources/${name}.rs"\n\n[dependencies]\n${dependency}`);
    const answer = name === "alpha" ? "helper::n() + beta::answer()" : "helper::n()";
    writeFileSync(join(dir, "sources", `${name}.rs`), `mod helper {\n    pub fn n() -> u32 {\n        1\n    }\n}\n\npub fn answer() -> u32 {\n    ${answer}\n}\n`);
  }
  const inSource = () => checkCargo({ manifestPath: join(dir, "Cargo.toml"), toolchain: pin, compiler, offline: true, packageName: "alpha", inSource: true });
  await expect(inSource()).rejects.toThrow(/crate `(alpha|beta)`'s module `helper` and crate `(alpha|beta)`'s module `helper` would both be .*sources\/helper\.js/);
  expect(readdirSync(join(dir, "sources")).sort()).toEqual(["alpha.rs", "beta.rs"]);
}, 600_000);

// The JS beside the Rust it's from, as ReScript writes it and a project
// commits it (ADR 0041): `src/api.rs` is `src/api.js`, importing the other
// crates' where they are too. What a module was, the module gone, goes.
test("a Cargo build's JS in source is beside each module's Rust, and follows the modules", async () => {
  const dir = fixture("cargo-in-source");
  writeFileSync(join(dir, "Cargo.toml"), '[workspace]\nmembers = ["app", "shared"]\nresolver = "2"\n');
  for (const name of ["app", "shared"]) mkdirSync(join(dir, name, "src"), { recursive: true });
  writeFileSync(join(dir, "shared", "Cargo.toml"), '[package]\nname = "shared"\nversion = "0.1.0"\nedition = "2024"\n');
  writeFileSync(join(dir, "shared", "src", "lib.rs"), "pub fn seven() -> u32 {\n    7\n}\n");
  writeFileSync(join(dir, "app", "Cargo.toml"), '[package]\nname = "app"\nversion = "0.1.0"\nedition = "2024"\n\n[dependencies]\nshared = { path = "../shared" }\n');
  const app = join(dir, "app", "src");
  writeFileSync(join(app, "lib.rs"), "mod extra;\n\npub fn value() -> u32 {\n    extra::more(shared::seven())\n}\n");
  writeFileSync(join(app, "extra.rs"), "pub fn more(n: u32) -> u32 {\n    n + 1\n}\n");
  const manifest = join(dir, "Cargo.toml");
  const inSource = () => checkCargo({ manifestPath: manifest, toolchain: pin, compiler, offline: true, packageName: "app", inSource: true });
  const { js } = await inSource();
  expect(js).toBe(join(app, "lib.js"));
  expect(readFileSync(js, "utf8")).toContain('from "../../shared/src/lib.js"');
  expect(existsSync(join(app, "extra.js"))).toBe(true);
  expect(run([node ?? "node", "--input-type=module", "--eval", `console.log((await import(${JSON.stringify(js)})).value());`]).trim()).toBe("8");
  // The module gone, its JS goes; and the root, with JSX now, is `lib.jsx`.
  rmSync(join(app, "extra.rs"));
  writeFileSync(join(app, "lib.rs"), "#![allow(non_snake_case)]\n\nuse react::jsx;\n\npub fn value() -> u32 {\n    shared::seven()\n}\n\npub fn View() -> react::JSX::Element {\n    jsx! { <b /> }\n}\n");
  writeFileSync(join(dir, "app", "Cargo.toml"), readFileSync(join(dir, "app", "Cargo.toml"), "utf8") + `react = { package = "rust-js-react", path = ${JSON.stringify(join(root, "react"))} }\n`);
  const again = await inSource();
  expect(again.js).toBe(join(app, "lib.jsx"));
  expect(existsSync(join(app, "extra.js"))).toBe(false);
  expect(existsSync(join(app, "lib.js"))).toBe(false);
  // A file of the project's own, not rust-js's, stays.
  writeFileSync(join(app, "notes.js"), "export const notes = 1;\n");
  await inSource();
  expect(existsSync(join(app, "notes.js"))).toBe(true);
}, 600_000);
// A crate's declarations (ADR 0196) are beside its Rust too, `App.d.ts`
// beside `App.js`, each module's, for the TypeScript that imports it.
test("a Cargo build's declarations in source are beside each module's JS", async () => {
  const dir = fixture("cargo-in-source-declarations");
  const src = join(dir, "src");
  mkdirSync(src, { recursive: true });
  writeFileSync(join(dir, "Cargo.toml"), '[package]\nname = "app"\nversion = "0.1.0"\nedition = "2024"\n\n[lib]\npath = "src/App.rs"\n\n[package.metadata.rust-js]\ndeclarations = true\n');
  writeFileSync(join(src, "App.rs"), "mod util;\n\npub fn answer(n: u32) -> u32 {\n    util::twice(n)\n}\n");
  writeFileSync(join(src, "util.rs"), "pub fn twice(n: u32) -> u32 {\n    n * 2\n}\n");
  await checkCargo({ manifestPath: join(dir, "Cargo.toml"), toolchain: pin, compiler, offline: true, packageName: "app", inSource: true });
  expect(readdirSync(src).filter((file) => file.endsWith(".d.ts")).sort()).toEqual(["App.d.ts", "util.d.ts"]);
  expect(readFileSync(join(src, "App.d.ts"), "utf8")).toContain("export function answer(n: number): number;");
}, 600_000);

// Where a module names another's file is where the compiler's own parse
// says, not where an import is printed (ADR 0101): after a directive, in an
// `export .. from`, and in a declaration, each names the root by the name
// it's copied to, `App.js`, not the build's `lib.js`.
test("a Cargo build's JS in source names each module's copy, wherever it names it", async () => {
  const dir = fixture("cargo-in-source-links");
  const src = join(dir, "src");
  mkdirSync(src, { recursive: true });
  writeFileSync(join(dir, "Cargo.toml"), '[package]\nname = "app"\nversion = "0.1.0"\nedition = "2024"\n\n[lib]\npath = "src/App.rs"\n\n[package.metadata.rust-js]\ndeclarations = true\n');
  writeFileSync(join(src, "App.rs"), "pub mod child;\npub mod plain;\n\npub struct Value {\n    pub n: u32,\n}\n\npub fn base() -> u32 {\n    1\n}\n");
  writeFileSync(join(src, "child.rs"), '#[cfg_attr(rust_js, rust_js::directive = "use client")]\nconst _: () = ();\n\npub fn twice() -> u32 {\n    crate::base() * 2\n}\n');
  writeFileSync(join(src, "plain.rs"), "pub use crate::base;\n\npub fn make() -> crate::Value {\n    crate::Value { n: 3 }\n}\n");
  await checkCargo({ manifestPath: join(dir, "Cargo.toml"), toolchain: pin, compiler, offline: true, packageName: "app", inSource: true });
  const child = readFileSync(join(src, "child.js"), "utf8");
  const plain = readFileSync(join(src, "plain.js"), "utf8");
  const types = readFileSync(join(src, "plain.d.ts"), "utf8");
  expect(child).toContain('"use client";\n\nimport { base } from "./App.js";');
  expect(plain).toContain('export { base } from "./App.js";');
  expect(types).toContain('from "./App.js";');
  for (const text of [child, plain, types]) expect(text).not.toContain("lib.js");
  const program = `const child = await import(${JSON.stringify(join(src, "child.js"))});
const plain = await import(${JSON.stringify(join(src, "plain.js"))});
console.log(child.twice(), plain.base(), plain.make().n);`;
  expect(run([node ?? "node", "--input-type=module", "--eval", program]).trim()).toBe("2 1 3");
}, 600_000);
