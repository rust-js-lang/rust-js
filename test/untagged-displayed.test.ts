// An untagged enum whose `Display` writes each variant's value as it is,
// webpack's module id of `string | number` (#557): every plain `{}` of it is
// the value in JS's template, `${id}`, so its impl is written only where
// something reaches it, a generic call given its dictionary; and its
// library's contract says so, so a consumer's `{}` of it is in place too,
// and one that needs the impl compiles its own (rd730, ADR 0214, amended).

import { beforeAll, expect, test } from "bun:test";
import { readFileSync, writeFileSync } from "node:fs";
import { join } from "node:path";
import { buildReact, compiler, fixture, run, target } from "./support";

beforeAll(buildReact, 600_000);

const js = ["--extern", `js=${join(target, "libjs.rmeta")}`, "-L", target];

/** `dep`, a library, and `app`, a crate that uses it, compiled in `dir`:
 * each one's JS. */
function compiled(dir: string, dep: string, app: string): { dep: string; app: string } {
  writeFileSync(join(dir, "dep.rs"), dep);
  run([compiler, join(dir, "dep.rs"), "-o", join(dir, "dep", "lib.js"), "--library", "--manifest", join(dir, "dep", "lib.manifest.json"),
    "--", "--crate-name", "dep", `--emit=metadata=${join(dir, "libdep.rmeta")}`, ...js]);
  writeFileSync(join(dir, "app.rs"), app);
  run([compiler, join(dir, "app.rs"), "-o", join(dir, "app", "lib.js"), "--dependency", join(dir, "dep", "lib.manifest.json"),
    "--", "--crate-name", "app", "--extern", `dep=${join(dir, "libdep.rmeta")}`, "-L", dir, ...js]);
  return { dep: readFileSync(join(dir, "dep", "lib.js"), "utf8"), app: readFileSync(join(dir, "app", "lib.js"), "utf8") };
}

const dep = `use std::fmt;

#[derive(Clone, PartialEq)]
#[cfg_attr(rust_js, rust_js::untagged)]
pub enum ModuleId {
    Name(String),
    Number(u32),
}

impl fmt::Display for ModuleId {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            ModuleId::Name(name) => write!(f, "{name}"),
            ModuleId::Number(number) => write!(f, "{number}"),
        }
    }
}

pub fn missing(id: &ModuleId) -> String {
    format!("Could not find the module \\"{id}\\"")
}

pub fn key(id: &ModuleId) -> String {
    id.to_string()
}
`;

// webpack's ReactFlightImportMetadata: each use prints in place, so the
// module has no impl of it, and the library's contract lists the type.
test("an untagged enum's Display each use prints in place is written nowhere", async () => {
  const dir = fixture("untagged-displayed-unwritten");
  const { dep: lib, app } = compiled(dir, dep, `use dep::ModuleId;
use std::fmt::Display;

fn shown<T: Display>(value: &T) -> String {
    value.to_string()
}

pub fn named(id: &ModuleId) -> String {
    format!("[{id}]")
}

pub fn padded(id: &ModuleId) -> String {
    format!("{id:>4}")
}

pub fn generic(id: &ModuleId) -> String {
    shown(id)
}
`);
  expect(lib).not.toContain("moduleIdDisplay");
  expect(lib).toContain('  return `Could not find the module "${id}"`;');
  expect(lib).toContain("  return `${id}`;");
  const manifest = JSON.parse(readFileSync(join(dir, "dep", "lib.manifest.json"), "utf8"));
  expect(manifest.library.displays_value).toHaveLength(1);
  // A consumer's plain `{}` is in place; the impl it calls with options or
  // through a dictionary is its own copy.
  expect(app).toContain("  return `[${id}]`;");
  expect(app).toContain("moduleIdDisplay_fmt");
  const { named, padded, generic } = await import(join(dir, "app", "lib.js"));
  const { missing, key } = await import(join(dir, "dep", "lib.js"));
  expect([named("a"), named(7), padded(7), padded("ab"), generic(12), generic("x")]).toEqual(["[a]", "[7]", "7", "ab", "12", "x"]);
  expect([missing(3), key("main")]).toEqual(['Could not find the module "3"', "main"]);
});

// One a generic call is given the dictionary of, in its own crate, is
// written, where it's reached.
test("an untagged enum's Display a generic call is given is written", async () => {
  const dir = fixture("untagged-displayed-reached");
  const { dep: lib } = compiled(dir, `${dep}
fn shown<T: fmt::Display>(value: &T) -> String {
    value.to_string()
}

pub fn generic(id: &ModuleId) -> String {
    shown(id)
}
`, "pub fn none() {}\n");
  expect(lib).toContain("const moduleIdDisplay = { fmt: moduleIdDisplay_fmt };");
  expect(lib).toContain("  return shown(id, moduleIdDisplay);");
  const { generic, key } = await import(join(dir, "dep", "lib.js"));
  expect([generic(5), generic("m"), key(6)]).toEqual(["5", "m", "6"]);
});
