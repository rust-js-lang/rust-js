// A checked function Rust calls is split in two (#2403 step 2b(a), ADR
// 1193): its JS entry, `f`, checks what a JS caller gives it and calls its
// body, `f$`, which the crate's own Rust calls, and a consumer's through
// its library's contract, so Rust never pays for JS's checks. A function
// Rust gives as a value stays `f`, as JS may call it; one only JS calls
// stays one function; and TypeScript sees `f` alone.
import { beforeAll, expect, test } from "bun:test";
import { mkdirSync, readFileSync, writeFileSync } from "node:fs";
import { join } from "node:path";
import { buildReact, compiler, fixture, run, target } from "./support";

const js = ["--extern", `js=${join(target, "libjs.rmeta")}`, "-L", target];

const dep = `pub fn double(n: u32) -> u32 {
    n * 2
}

pub fn quadruple(n: u32) -> u32 {
    double(double(n))
}

// Only JS calls it in this crate, but a library's consumers may: split.
pub fn alone(n: u32) -> u32 {
    n + 1
}

pub fn as_value() -> fn(u32) -> u32 {
    double
}
`;

const app = `pub fn eight(n: u32) -> u32 {
    dep::double(dep::quadruple(n))
}

// A program's own function only JS calls: one function.
pub fn own(n: u32) -> u32 {
    n + 2
}

pub fn twice_own(n: u32) -> u32 {
    triple(n) + triple(n)
}

pub fn triple(n: u32) -> u32 {
    n * 3
}
`;

let dir: string;
let depJs: string;
let appJs: string;
let depDts: string;
let lib: Record<string, (...args: unknown[]) => unknown>;
let deps: Record<string, (...args: unknown[]) => unknown>;

beforeAll(buildReact, 600_000);

beforeAll(async () => {
  dir = fixture("boundary-split");
  mkdirSync(join(dir, "dep"), { recursive: true });
  mkdirSync(join(dir, "app"), { recursive: true });
  writeFileSync(join(dir, "dep.rs"), dep);
  run([compiler, "--declarations", join(dir, "dep.rs"), "-o", join(dir, "dep", "lib.js"), "--library", "--manifest", join(dir, "dep", "lib.manifest.json"),
    "--", "--crate-name", "dep", `--emit=metadata=${join(dir, "libdep.rmeta")}`, ...js]);
  writeFileSync(join(dir, "app.rs"), app);
  run([compiler, join(dir, "app.rs"), "-o", join(dir, "app", "lib.js"), "--dependency", join(dir, "dep", "lib.manifest.json"),
    "--", "--crate-name", "app", "--extern", `dep=${join(dir, "libdep.rmeta")}`, "-L", dir, ...js]);
  depJs = readFileSync(join(dir, "dep", "lib.js"), "utf8");
  appJs = readFileSync(join(dir, "app", "lib.js"), "utf8");
  depDts = readFileSync(join(dir, "dep", "lib.d.ts"), "utf8");
  deps = await import(join(dir, "dep", "lib.js"));
  lib = await import(join(dir, "app", "lib.js"));
}, 600_000);

const thrown = (call: () => unknown) => {
  try {
    call();
  } catch (error) {
    return `${(error as Error).name}: ${(error as Error).message}`;
  }
  return "nothing thrown";
};

test("a library's checked function is its entry, which checks, and its body, which Rust calls", () => {
  expect(depJs).toContain('export function double(n) {\n  $checkU32(n, "double(n)");\n  return double$(n);\n}');
  // Its body, past the checks its entry makes.
  expect(depJs).toContain("export function double$(n) {\n  return (n * 2) >>> 0;\n}");
  expect(depJs).toContain("return double$(double$(n));");
  expect(depJs).toContain("export function alone$(n) {");
  expect(deps.quadruple!(3)).toBe(12);
  expect(thrown(() => deps.double!(-1))).toBe("RangeError: rust-js: double(n): expected a u32, got -1");
});

test("a function Rust gives as a value is its entry, as JS may call it", () => {
  expect(depJs).toMatch(/export function asValue\(\) \{\n {2}return double;\n\}/);
  const given = deps.asValue!() as (n: unknown) => unknown;
  expect(thrown(() => given(-1))).toBe("RangeError: rust-js: double(n): expected a u32, got -1");
});

test("a consumer's direct call calls the body its library's contract names", () => {
  const manifest = JSON.parse(readFileSync(join(dir, "dep", "lib.manifest.json"), "utf8"));
  const double = manifest.library.items.find((item: { export: string }) => item.export === "double");
  expect(double.unchecked).toBe("double$");
  // Imported by its body's name, named as the module reads it.
  // Through the library's module, as `dep::double` names it, as its entry
  // is read (rd114).
  expect(appJs).toContain('import * as Dep from "../dep/lib.js";');
  expect(appJs).toContain("return Dep.double$(Dep.quadruple$(n));");
  expect(lib.eight!(2)).toBe(16);
});

test("a program's function only JS calls is one function; one Rust calls too is split, its body private", () => {
  expect(appJs).toMatch(/export function own\(n\) \{\n {2}\$checkU32\(n, "own\(n\)"\);\n {2}return/);
  expect(appJs).not.toContain("own$");
  expect(appJs).toContain("function triple$(n) {");
  expect(appJs).not.toContain("export function triple$");
  expect(appJs).toContain("triple$(n) + triple$(n)");
});

test("TypeScript sees the entry alone", () => {
  expect(depDts).toContain("export function double(n: number): number;");
  expect(depDts).not.toContain("double$");
  expect(depDts).not.toContain("alone$");
});

// A split function's call is the body where Rust names it directly, even
// by a `use` that renames it, `close as close_flight`, which its import
// keeps: `import { close$ as closeFlight }`. Through another module that
// re-exports it, `plugins::read(..)`, it's that module's entry, as its
// file re-exports no body (rd137), so what Rust gives it is what Rust
// made, branded, which the entry's check takes (ADR 1196).
test("a split function's call through a renaming use is its body, and through a re-exporting module its entry", async () => {
  const at = fixture("boundary-split-paths");
  const files: Record<string, string> = {
    "lib.rs": `pub mod client;
pub mod markup;
pub mod plugins;
pub mod shared;

pub use client::close;
pub use shared::read;

pub fn run(n: u32) -> u32 {
    plugins::read(&shared::Token(n)) + markup::render(n)
}
`,
    "client.rs": "pub fn close(n: u32) -> u32 {\n    n + 1\n}\n",
    "markup.rs": "use crate::client::close as close_flight;\n\npub fn render(n: u32) -> u32 {\n    close_flight(n) * 10\n}\n",
    "plugins.rs": "pub use crate::shared::read;\n",
    "shared.rs": "pub struct Token(pub(crate) u32);\n\npub fn read(token: &Token) -> u32 {\n    token.0 + 1\n}\n",
  };
  for (const [file, text] of Object.entries(files)) writeFileSync(join(at, file), text);
  run([compiler, join(at, "lib.rs"), "-o", join(at, "lib.js"), "--", "--crate-type=lib", "--edition=2024"]);
  const markup = readFileSync(join(at, "markup.js"), "utf8");
  expect(markup).toContain('import { close$ as closeFlight } from "./client.js";\n');
  expect(markup).toContain("return (closeFlight(n) * 10) >>> 0;");
  const root = readFileSync(join(at, "lib.js"), "utf8");
  expect(root).toContain('import * as Plugins from "./plugins.js";\n');
  expect(root).toMatch(/Plugins\.read\(\$brand\("lib#[0-9a-f]+::shared::Token", \[n\]\)\)/);
  const made = await import(join(at, "lib.js"));
  expect(made.run(2)).toBe(3 + 30);
  expect(thrown(() => made.read([2]))).toBe("TypeError: rust-js: read(token): expected a Token Rust made, got an array");
});
