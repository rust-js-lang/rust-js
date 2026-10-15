// A rust-js library on npm, as ReScript's are (ADR 0414): one package of its
// Rust and the JS its author built, which a JS app imports as it is, and a
// rust-js app compiles again, by its own rust-js, into the package, where
// its package manager put it, npm's way or pnpm's, and imports by the
// package's name.

import { beforeAll, expect, test } from "bun:test";
import { cpSync, linkSync, mkdirSync, readFileSync, readdirSync, symlinkSync, writeFileSync } from "node:fs";
import { dirname, join } from "node:path";
import { runSync } from "./child";
import { node } from "./programs";
import { buildCompiler, compiler, fixture, root, run } from "./support";

beforeAll(buildCompiler, 600_000);

const cli = join(root, "tooling", "cli.js");
const build = (dir: string, ...args: string[]) => runSync([process.execPath, cli, "build", ...args], dir, 300_000, { RUST_JS_COMPILER: compiler });

/** The library as its author has it, built by `rust-js build` and ready to pack. */
function library() {
  const dir = fixture("library");
  mkdirSync(join(dir, "src"));
  writeFileSync(join(dir, "package.json"), JSON.stringify({
    name: "@acme/greet",
    version: "1.0.0",
    main: "src/lib.js",
    types: "src/lib.d.ts",
    "rust-js": { crate: "acme-greet" },
  }));
  writeFileSync(join(dir, "Cargo.toml"), '[package]\nname = "acme-greet"\nversion = "1.0.0"\nedition = "2024"\n\n[lib]\nname = "greet"\n\n[workspace]\n');
  writeFileSync(join(dir, "src", "lib.rs"), `pub struct Greeting {
    pub name: String,
}

impl Greeting {
    pub fn text(&self) -> String {
        format!("Hello, {}!", self.name)
    }
}

pub fn greet(name: &str) -> Greeting {
    Greeting { name: name.to_string() }
}
`);
  const built = build(dir);
  expect(built.stderr).toBe("");
  expect(built.code).toBe(0);
  return dir;
}

/** An app that depends on the library, in npm and in Cargo, with `@rust-js/runtime` installed. */
function app(name: string) {
  const dir = fixture(name);
  mkdirSync(join(dir, "src"));
  writeFileSync(join(dir, "package.json"), JSON.stringify({ private: true, dependencies: { "@acme/greet": "1.0.0" } }));
  writeFileSync(join(dir, "Cargo.toml"), '[package]\nname = "app"\nversion = "0.0.0"\nedition = "2024"\n\n[dependencies]\ngreet = { package = "acme-greet", version = "1" }\n\n[workspace]\n');
  writeFileSync(join(dir, "src", "lib.rs"), 'pub fn welcome() -> String {\n    greet::greet("Ada").text()\n}\n');
  mkdirSync(join(dir, "node_modules", "@rust-js"), { recursive: true });
  symlinkSync(join(root, "runtime"), join(dir, "node_modules", "@rust-js", "runtime"));
  return dir;
}

/** What npm packs of `dir`: its manifests, its Rust and its JS. */
function packed(dir: string, to: string, copy: (from: string, to: string) => void) {
  for (const file of ["package.json", "Cargo.toml", ...readdirSync(join(dir, "src")).filter((f) => !f.endsWith(".map")).map((f) => join("src", f))]) {
    mkdirSync(dirname(join(to, file)), { recursive: true });
    copy(join(dir, file), join(to, file));
  }
}

const welcome = (dir: string) =>
  run([node ?? "node", "--input-type=module", "--eval", `console.log((await import(${JSON.stringify(join(dir, "src", "lib.js"))})).welcome());`]).trim();

// npm's way, and bun's: the package in the app's one `node_modules`.
test("a rust-js app compiles a library from npm into its package, and imports it by the package's name", () => {
  const lib = library();
  const dir = app("library-npm");
  const at = join(dir, "node_modules", "@acme", "greet");
  packed(lib, at, cpSync);
  // Built by another rust-js than the app's, its JS isn't what the app's writes.
  writeFileSync(join(at, "src", "lib.js"), `${readFileSync(join(at, "src", "lib.js"), "utf8")}// an older rust-js's\n`);
  const built = build(dir);
  expect(built.stderr).toBe("");
  expect(built.code).toBe(0);
  const js = readFileSync(join(dir, "src", "lib.js"), "utf8");
  expect(js).toContain('from "@acme/greet/src/lib.js";');
  expect(js).not.toContain("node_modules");
  expect(readFileSync(join(at, "src", "lib.js"), "utf8")).toBe(readFileSync(join(lib, "src", "lib.js"), "utf8"));
  expect(welcome(dir)).toBe("Hello, Ada!");
  // What's in `node_modules` isn't the app's to commit: `--check` is of its own.
  writeFileSync(join(at, "src", "lib.js"), `${readFileSync(join(at, "src", "lib.js"), "utf8")}// an older rust-js's\n`);
  const checked = build(dir, "--check");
  expect(checked.stderr).toBe("");
  expect(checked.code).toBe(0);
}, 600_000);

// pnpm's way: the package in its store, each file a hard link to the store's,
// which other projects share, and linked into the app's `node_modules`.
test("a library from pnpm's store is compiled into its package without changing the store's files", () => {
  const lib = library();
  const dir = app("library-pnpm");
  const store = join(fixture("library-store"), "greet");
  packed(lib, store, cpSync);
  writeFileSync(join(store, "src", "lib.js"), `${readFileSync(join(store, "src", "lib.js"), "utf8")}// an older rust-js's\n`);
  const shipped = readFileSync(join(store, "src", "lib.js"), "utf8");
  const at = join(dir, "node_modules", ".pnpm", "@acme+greet@1.0.0", "node_modules", "@acme", "greet");
  packed(store, at, linkSync);
  mkdirSync(join(dir, "node_modules", "@acme"));
  symlinkSync(at, join(dir, "node_modules", "@acme", "greet"));
  const built = build(dir);
  expect(built.stderr).toBe("");
  expect(built.code).toBe(0);
  const js = readFileSync(join(dir, "src", "lib.js"), "utf8");
  expect(js).toContain('from "@acme/greet/src/lib.js";');
  expect(js).not.toContain(".pnpm");
  expect(welcome(dir)).toBe("Hello, Ada!");
  // The store's file is as it was: the package's was replaced, not written through.
  expect(readFileSync(join(store, "src", "lib.js"), "utf8")).toBe(shipped);
  expect(readFileSync(join(at, "src", "lib.js"), "utf8")).toBe(readFileSync(join(lib, "src", "lib.js"), "utf8"));
}, 600_000);

// A JS app has no Rust: it imports the JS the author built, as it is.
test("a JS app imports a rust-js library's JS as its author built it", () => {
  const lib = library();
  const dir = fixture("library-js");
  writeFileSync(join(dir, "package.json"), JSON.stringify({ private: true, type: "module", dependencies: { "@acme/greet": "1.0.0" } }));
  packed(lib, join(dir, "node_modules", "@acme", "greet"), cpSync);
  mkdirSync(join(dir, "node_modules", "@rust-js"), { recursive: true });
  symlinkSync(join(root, "runtime"), join(dir, "node_modules", "@rust-js", "runtime"));
  // A method is its type's function, given the value (ADR 0047).
  writeFileSync(join(dir, "main.js"), 'import { Greeting, greet } from "@acme/greet";\nconsole.log(Greeting.text(greet("Ada")));\n');
  expect(run([node ?? "node", join(dir, "main.js")]).trim()).toBe("Hello, Ada!");
}, 600_000);
