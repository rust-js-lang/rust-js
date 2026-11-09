// A crate named as one std is made of, `cfg_if`: std's sysroot has its own,
// loaded beside std, and a program's of that name is another crate, its
// library the build's (#1130). A git repository stands in for crates.io.

import { beforeAll, expect, test } from "bun:test";
import { mkdirSync, readFileSync, writeFileSync } from "node:fs";
import { join } from "node:path";
import { runSync } from "./child";
import { node } from "./programs";
import { buildCompiler, compiler, fixture, root, run } from "./support";

beforeAll(buildCompiler, 600_000);

test("a crate named as one std is made of, cfg_if, is the program's, compiled and called", () => {
  const dir = fixture("sysroot-names");
  const crate = join(dir, "cfg-if");
  mkdirSync(join(crate, "src"), { recursive: true });
  writeFileSync(join(crate, "Cargo.toml"), '[package]\nname = "cfg-if"\nversion = "1.0.5"\nedition = "2024"\n');
  writeFileSync(join(crate, "src", "lib.rs"), "#![no_std]\n\n#[macro_export]\nmacro_rules! cfg_if {\n    ($($t:tt)*) => {\n        pub fn arch() -> u32 {\n            32\n        }\n    };\n}\n\npub fn one() -> u32 {\n    1\n}\n");
  const git = (...args: string[]) => expect(runSync(["git", "-c", "user.name=t", "-c", "user.email=t@t", ...args], crate, 60_000).code).toBe(0);
  git("init", "-q");
  git("add", ".");
  git("commit", "-q", "-m", "cfg-if");
  const app = join(dir, "app");
  mkdirSync(join(app, "src"), { recursive: true });
  writeFileSync(join(app, "Cargo.toml"), `[package]\nname = "app"\nversion = "0.0.0"\nedition = "2024"\n\n[dependencies]\ncfg-if = { git = ${JSON.stringify(`file://${crate}`)} }\n\n[workspace]\n`);
  writeFileSync(join(app, "src", "lib.rs"), "cfg_if::cfg_if! {}\n\npub fn sum() -> u32 {\n    arch() + cfg_if::one()\n}\n");
  const built = runSync([process.execPath, join(root, "tooling", "cli.js"), "build"], app, 300_000, { RUST_JS_COMPILER: compiler });
  expect(built.stderr).toBe("");
  expect(built.code).toBe(0);
  expect(readFileSync(join(app, "src", "lib.js"), "utf8")).toContain('from "../rust-js/crates/cfg-if-1.0.5/src/lib.js"');
  const program = `console.log((await import(${JSON.stringify(join(app, "src", "lib.js"))})).sum());`;
  expect(run([node ?? "node", "--input-type=module", "--eval", program]).trim()).toBe("33");
}, 600_000);

// Two releases of one crate in one graph, each a library of its own name,
// told apart by its identity, `StableCrateId`, as the key a consumer knows
// a library by is its name and identity, never its name alone (#1130).
test("two releases of one crate, cfg-if 0.1 and 1.0, are two libraries of one build", () => {
  const dir = fixture("crate-names-twice");
  const release = (version: string, one: number) => {
    const crate = join(dir, `cfg-if-${version}`);
    mkdirSync(join(crate, "src"), { recursive: true });
    writeFileSync(join(crate, "Cargo.toml"), `[package]\nname = "cfg-if"\nversion = "${version}"\nedition = "2024"\n`);
    writeFileSync(join(crate, "src", "lib.rs"), `pub fn one() -> u32 {\n    ${one}\n}\n`);
    const git = (...args: string[]) => expect(runSync(["git", "-c", "user.name=t", "-c", "user.email=t@t", ...args], crate, 60_000).code).toBe(0);
    git("init", "-q");
    git("add", ".");
    git("commit", "-q", "-m", version);
    return crate;
  };
  const [old, now] = [release("0.1.10", 10), release("1.0.5", 1)];
  const app = join(dir, "app");
  mkdirSync(join(app, "src"), { recursive: true });
  writeFileSync(join(app, "Cargo.toml"), `[package]\nname = "app"\nversion = "0.0.0"\nedition = "2024"\n\n[dependencies]\nold = { package = "cfg-if", git = ${JSON.stringify(`file://${old}`)} }\nnow = { package = "cfg-if", git = ${JSON.stringify(`file://${now}`)} }\n\n[workspace]\n`);
  writeFileSync(join(app, "src", "lib.rs"), "pub fn sum() -> u32 {\n    old::one() + now::one()\n}\n");
  const built = runSync([process.execPath, join(root, "tooling", "cli.js"), "build"], app, 300_000, { RUST_JS_COMPILER: compiler });
  expect(built.stderr).toBe("");
  expect(built.code).toBe(0);
  const program = `console.log((await import(${JSON.stringify(join(app, "src", "lib.js"))})).sum());`;
  expect(run([node ?? "node", "--input-type=module", "--eval", program]).trim()).toBe("11");
}, 600_000);
