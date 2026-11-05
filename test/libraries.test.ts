// A rust-js library on npm, as ReScript's are (ADR 0414): one package of its
// Rust and the JS its author built, which a JS app imports as it is, and a
// rust-js app compiles again, by its own rust-js, into the package, where
// its package manager put it, npm's way or pnpm's, and imports by the
// package's name.

import { beforeAll, expect, test } from "bun:test";
import { cpSync, existsSync, linkSync, mkdirSync, mkdtempSync, readFileSync, readdirSync, realpathSync, rmSync, symlinkSync, writeFileSync } from "node:fs";
import { tmpdir } from "node:os";
import { dirname, join } from "node:path";
import { runSync } from "./child";
import { node } from "./programs";
import { buildCompiler, compiler, fixture, root, run, runtimeDir, stdDir } from "./support";

beforeAll(buildCompiler, 600_000);

const cli = join(root, "tooling", "cli.js");
const build = (dir: string, ...args: string[]) => runSync([process.execPath, cli, "build", ...args], dir, 300_000, { RUST_JS_COMPILER: compiler });

/** The library as its author has it, built by `rust-js build` and ready to pack, its `package.json` with `exports` if given, and `more` Rust. */
function library(exports?: Record<string, string>, more = "") {
  const dir = fixture("library");
  mkdirSync(join(dir, "src"));
  writeFileSync(join(dir, "package.json"), JSON.stringify({
    name: "@acme/greet",
    version: "1.0.0",
    main: "src/lib.js",
    types: "src/lib.d.ts",
    "rust-js": { crate: "acme-greet" },
    exports,
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
${more}`);
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
  symlinkSync(runtimeDir, join(dir, "node_modules", "@rust-js", "runtime"));
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
  expect(js).toContain('import { Greeting } from "@acme/greet";\nimport * as Greet from "@acme/greet";\n');
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
  expect(js).toContain('import { Greeting } from "@acme/greet";\nimport * as Greet from "@acme/greet";\n');
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
  symlinkSync(runtimeDir, join(dir, "node_modules", "@rust-js", "runtime"));
  // A method is its type's function, given the value (ADR 0047).
  writeFileSync(join(dir, "main.js"), 'import { Greeting, greet } from "@acme/greet";\nconsole.log(Greeting.text(greet("Ada")));\n');
  expect(run([node ?? "node", join(dir, "main.js")]).trim()).toBe("Hello, Ada!");
}, 600_000);

// The app imports a package's root by its name, `@acme/greet`, and another
// module's JS by its path, `@acme/greet/src/loud.js`: a package with
// `exports` exports it too, or the build says what to export (ADR 1008,
// amended).
test("a library whose exports hide its JS is refused, and one exporting it is imported by path", () => {
  const loud = "\npub mod loud {\n    pub fn shout(text: &str) -> String {\n        text.to_uppercase()\n    }\n}\n";
  for (const [exports, ok] of [[{ ".": "./src/lib.js" }, false], [{ ".": "./src/lib.js", "./src/*": "./src/*" }, true]] as const) {
    const lib = library(exports, loud);
    const dir = app("library-exports");
    writeFileSync(join(dir, "src", "lib.rs"), 'pub fn welcome() -> String {\n    greet::greet(&greet::loud::shout("Ada")).text()\n}\n');
    packed(lib, join(dir, "node_modules", "@acme", "greet"), cpSync);
    const built = build(dir);
    if (ok) {
      expect(built.stderr).toBe("");
      const js = readFileSync(join(dir, "src", "lib.js"), "utf8");
      expect(js).toContain('import { Greeting } from "@acme/greet";\nimport * as Greet from "@acme/greet";\n');
      expect(js).toContain('import * as Loud from "@acme/greet/src/loud.js";\n');
      expect(welcome(dir)).toBe("Hello, ADA!");
    } else {
      expect(built.code).not.toBe(0);
      expect(built.stderr).toContain("@acme/greet's `exports` hides ./src/loud.js, which the app's JS imports, `@acme/greet/src/loud.js`: export it, `\"./src/*\": \"./src/*\"`");
    }
  }
}, 600_000);

// A package's crate is quiet, as ReScript's dependencies are: what it warns
// of is its author's, which the app can't act on; the app's own is said.
test("a failed build says the app's errors, not what a library's crate warns of", () => {
  const lib = library(undefined, "\npub fn unused() {\n    let spare = 1;\n}\n");
  const dir = app("library-quiet");
  packed(lib, join(dir, "node_modules", "@acme", "greet"), cpSync);
  writeFileSync(join(dir, "src", "lib.rs"), 'pub fn welcome() -> String {\n    greet::greet("Ada").text() + 1\n}\n');
  const built = build(dir);
  expect(built.code).not.toBe(0);
  expect(built.stderr).toContain("src/lib.rs:2");
  expect(built.stderr).not.toContain("spare");
}, 600_000);

// As a user has it: the library packed by `npm pack`, its JS an older
// rust-js's, and installed by npm, bun and pnpm, each as it installs, pnpm's
// store linking each file into the app. Outside this checkout, so no
// workspace of its is the app's.
test("a library packed by npm is built into an app by each package manager's install", () => {
  const lib = library();
  writeFileSync(join(lib, "src", "lib.js"), `${readFileSync(join(lib, "src", "lib.js"), "utf8")}// an older rust-js's\n`);
  const work = realpathSync(mkdtempSync(join(tmpdir(), "rust-js libraries ")));
  const packedBy = runSync(["npm", "pack", "--pack-destination", work], lib, 120_000);
  expect(packedBy.stderr).not.toContain("ERR");
  const tarball = join(work, "acme-greet-1.0.0.tgz");
  const env = { COREPACK_ENABLE_DOWNLOAD_PROMPT: "0", COREPACK_ENABLE_STRICT: "0" };
  const installs: Record<string, string[]> = {
    npm: ["npm", "install", "--no-audit", "--no-fund"],
    bun: [process.execPath, "install"],
    pnpm: ["corepack", "pnpm@10.0.0", "install"],
  };
  for (const [manager, install] of Object.entries(installs)) {
    const dir = join(work, manager);
    mkdirSync(join(dir, "src"), { recursive: true });
    writeFileSync(join(dir, "package.json"), JSON.stringify({ private: true, dependencies: { "@acme/greet": `file:${tarball}` } }));
    writeFileSync(join(dir, "Cargo.toml"), '[package]\nname = "app"\nversion = "0.0.0"\nedition = "2024"\n\n[dependencies]\ngreet = { package = "acme-greet", version = "1" }\n\n[workspace]\n');
    writeFileSync(join(dir, "src", "lib.rs"), 'pub fn welcome() -> String {\n    greet::greet("Ada").text()\n}\n');
    const installed = runSync(install, dir, 300_000, env);
    expect({ manager, code: installed.code, stderr: installed.code ? installed.stderr : "" }).toEqual({ manager, code: 0, stderr: "" });
    mkdirSync(join(dir, "node_modules", "@rust-js"), { recursive: true });
    symlinkSync(runtimeDir, join(dir, "node_modules", "@rust-js", "runtime"), "dir");
    symlinkSync(stdDir, join(dir, "node_modules", "@rust-js", "std"), "dir");
    // A witness of the installed file: one written through is seen in it.
    const installedJs = realpathSync(join(dir, "node_modules", "@acme", "greet", "src", "lib.js"));
    const shipped = readFileSync(installedJs, "utf8");
    linkSync(installedJs, join(work, `${manager}-witness.js`));
    const built = build(dir);
    expect({ manager, code: built.code, stderr: built.stderr }).toEqual({ manager, code: 0, stderr: "" });
    expect(readFileSync(join(dir, "src", "lib.js"), "utf8")).toContain('import { Greeting } from "@acme/greet";\nimport * as Greet from "@acme/greet";\n');
    expect(welcome(dir)).toBe("Hello, Ada!");
    expect(readFileSync(installedJs, "utf8")).not.toContain("an older rust-js's");
    expect({ manager, witness: readFileSync(join(work, `${manager}-witness.js`), "utf8") }).toEqual({ manager, witness: shipped });
    const checked = build(dir, "--check");
    expect({ manager, code: checked.code, stderr: checked.stderr }).toEqual({ manager, code: 0, stderr: "" });
  }
}, 600_000);

// A package's entry, React's `index.js`, `export {Children} from
// './src/ReactClient'`, is what a consumer imports a library's item by,
// where its root's default entry re-exports it from its module and the
// crate's Rust names it through the library's root, by the package's name
// (ADR 1008, amended): `import {Children} from 'react'`, one module
// instance with what's imported by its file; one named by its module's
// path, by its file. An entry that isn't JS, Flow's types in it, is
// refused, naming it.
test("an item a package's entry re-exports is imported by the package's name", () => {
  const dir = fixture("package-entry");
  writeFileSync(join(dir, "Cargo.toml"), '[workspace]\nmembers = ["kids", "dom"]\nresolver = "3"\n');
  for (const [name, more] of [["kids", ""], ["dom", '\n[dependencies]\nkids = { path = "../kids" }\n']]) {
    mkdirSync(join(dir, name, "src"), { recursive: true });
    writeFileSync(join(dir, name, "Cargo.toml"), `[package]\nname = "${name}"\nversion = "0.1.0"\nedition = "2024"\n${more}`);
  }
  const kids = join(dir, "kids");
  const manifest = (exports?: unknown) =>
    writeFileSync(join(kids, "package.json"), JSON.stringify({ name: "@acme/kids", type: "module", main: "index.js", exports }));
  manifest();
  writeFileSync(join(kids, "src", "lib.rs"), "pub mod client;\npub mod hooks;\n\npub use client::bump;\npub use hooks::hook;\n");
  writeFileSync(join(kids, "src", "client.rs"), `use std::cell::Cell;

thread_local! {
    static BUMPS: Cell<u32> = const { Cell::new(0) };
}

pub fn bump() {
    BUMPS.with(|bumps| bumps.set(bumps.get() + 1));
}

pub fn count() -> u32 {
    BUMPS.with(Cell::get)
}
`);
  writeFileSync(join(kids, "src", "hooks.rs"), "pub fn hook() -> u32 {\n    7\n}\n\npub fn other() -> u32 {\n    0\n}\n");
  const entry = "export {bump} from './src/client.js';\nexport * from './src/hooks.js';\n";
  writeFileSync(join(kids, "index.js"), entry);
  writeFileSync(join(dir, "dom", "src", "by_module.rs"), "pub fn bumped() -> u32 {\n    kids::client::bump();\n    kids::client::count()\n}\n");
  writeFileSync(join(dir, "dom", "src", "lib.rs"), "pub mod by_module;\n\npub fn run() -> u32 {\n    kids::bump();\n    by_module::bumped() * 10 + kids::hook() + kids::hooks::other()\n}\n");
  mkdirSync(join(dir, "node_modules", "@acme"), { recursive: true });
  symlinkSync(kids, join(dir, "node_modules", "@acme", "kids"), "dir");
  mkdirSync(join(dir, "node_modules", "@rust-js"));
  symlinkSync(runtimeDir, join(dir, "node_modules", "@rust-js", "runtime"), "dir");
  const js = () => readFileSync(join(dir, "dom", "src", "lib.js"), "utf8");
  const ran = () => run([node ?? "node", "--input-type=module", "--eval", `console.log((await import(${JSON.stringify(join(dir, "dom", "src", "lib.js"))})).run());`]).trim();
  const built = () => build(join(dir, "dom"));
  // What native Rust prints of the two crates.
  const native = join(dir, "native");
  mkdirSync(native);
  run(["rustc", "--edition=2024", "--crate-type=rlib", "--crate-name", "kids", join(kids, "src", "lib.rs"), "-o", join(native, "libkids.rlib")]);
  run(["rustc", "--edition=2024", "--crate-type=rlib", "--crate-name", "dom", join(dir, "dom", "src", "lib.rs"), "-o", join(native, "libdom.rlib"), "--extern", `kids=${join(native, "libkids.rlib")}`]);
  writeFileSync(join(native, "main.rs"), "fn main() {\n    println!(\"{}\", dom::run());\n}\n");
  run(["rustc", "--edition=2024", join(native, "main.rs"), "-o", join(native, "program"), "--extern", `dom=${join(native, "libdom.rlib")}`, "-L", `dependency=${native}`]);
  const nativeRun = run([join(native, "program")]).trim();
  expect(nativeRun).toBe("27");
  expect(built().stderr).toBe("");
  expect(js()).toContain('import { bump, hook } from "@acme/kids";\n');
  // One its module's path names, its namespace's, by its file (ADR 1018).
  expect(js()).toContain('import * as Hooks from "../../kids/src/hooks.js";\n');
  // Each module as its own `use` names it: this one by the module's path,
  // each of its items, not by the package's name the root's names it by.
  const byModule = readFileSync(join(dir, "dom", "src", "by_module.js"), "utf8");
  expect(byModule).toContain('import * as Client from "../../kids/src/client.js";\n');
  expect(byModule).toContain("  Client.bump();\n  return Client.count();\n");
  expect(byModule).not.toContain("@acme/kids");
  expect(ran()).toBe(nativeRun);
  // Its root re-exports what the entry does, each from the same module,
  // and nothing else: no JS of its own, the entry's already (ADR 1008,
  // amended), as React's react.rs is no react.js.
  const rootJs = join(kids, "src", "lib.js");
  expect(existsSync(rootJs)).toBe(false);
  // The default condition's entry alone, another the host's to choose
  // (ADR 0262's exception), the library rebuilt as its package.json changes.
  writeFileSync(join(kids, "server.js"), "export * from './src/hooks.js';\n");
  manifest({ ".": { browser: "./index.js", default: "./server.js" } });
  expect(built().stderr).toBe("");
  expect(js()).toContain('import { hook } from "@acme/kids";\n');
  expect(js()).toContain('import { bump } from "../../kids/src/client.js";\n');
  expect(ran()).toBe(nativeRun);
  // An entry that runs more than its re-exports as it loads, which the
  // Rust doesn't, and a package Node finds no copy of from the crate's JS,
  // or another: each by its file, as the Rust has it.
  const byFiles = 'import { bump } from "../../kids/src/client.js";\nimport { hook } from "../../kids/src/hooks.js";\nimport * as Hooks from "../../kids/src/hooks.js";\n';
  manifest();
  writeFileSync(join(kids, "index.js"), `${entry}console.log("loaded");\n`);
  expect(built().stderr).toBe("");
  expect(js()).toContain(byFiles);
  // Whose re-exports are then its root's own, as before.
  expect(readFileSync(rootJs, "utf8")).toContain("export { bump } from \"./client.js\";\n");
  writeFileSync(join(kids, "index.js"), entry);
  rmSync(join(dir, "node_modules", "@acme", "kids"));
  expect(built().stderr).toBe("");
  expect(js()).toContain(byFiles);
  expect(existsSync(rootJs)).toBe(false);
  // A root with an item of its own writes its JS, a type's too.
  writeFileSync(join(kids, "src", "lib.rs"), `${readFileSync(join(kids, "src", "lib.rs"), "utf8")}\npub type Count = u32;\n`);
  expect(built().stderr).toBe("");
  expect(readFileSync(rootJs, "utf8")).toContain("export { bump } from \"./client.js\";\n");
  expect(readFileSync(join(kids, "src", "lib.d.ts"), "utf8")).toContain("export type Count = number;");
  expect(ran()).toBe(nativeRun);
  // Flow's, which oxc doesn't read.
  writeFileSync(join(kids, "index.js"), `export type Kid = number;\n${entry}`);
  const refused = built();
  expect(refused.code).not.toBe(0);
  expect(refused.stderr).toContain(`rust-js can't read \`${realpathSync(join(kids, "index.js"))}\`, package \`@acme/kids\`'s entry, as JavaScript`);
  expect(refused.stderr).toContain("keep only JS in it, its types in a `.d.ts` beside it, `index.d.ts`");
  // Upstream's header, which says `@flow`, is kept: the entry is JS.
  symlinkSync(kids, join(dir, "node_modules", "@acme", "kids"), "dir");
  writeFileSync(join(kids, "index.js"), `/**\n * @flow\n */\n\n${entry}`);
  expect(built().stderr).toBe("");
  expect(js()).toContain('import { bump, hook } from "@acme/kids";\n');
}, 600_000);

// An entry's re-export of a module's own `pub use`, React's `index.js`
// `export {createElement} from './src/ReactClient'`, whose ReactClient.rs
// re-exports it of `jsx/ReactJSXElement.rs`, is of the item the `pub use`
// names: a consumer naming it through the library's root imports it by
// the package's name, `require('react').createElement`, as it does one of
// the entry's module's own, through each module that re-exports it in
// turn; and a root re-exporting it alone, a `pub use` of a list too,
// `pub use react_client::{CHILDREN, create_element};`, writes no JS of its
// own, the entry's already (rd755, ADR 1008, amended).
test("an item a package's entry re-exports through its module's re-export is imported by the package's name", () => {
  const dir = fixture("package-entry-through");
  writeFileSync(join(dir, "Cargo.toml"), '[workspace]\nmembers = ["kids", "dom"]\nresolver = "3"\n');
  for (const [name, more] of [["kids", ""], ["dom", '\n[dependencies]\nkids = { path = "../kids" }\n']]) {
    mkdirSync(join(dir, name, "src"), { recursive: true });
    writeFileSync(join(dir, name, "Cargo.toml"), `[package]\nname = "${name}"\nversion = "0.1.0"\nedition = "2024"\n${more}`);
  }
  const kids = join(dir, "kids");
  writeFileSync(join(kids, "package.json"), JSON.stringify({ name: "@acme/kids", type: "module", main: "index.js" }));
  writeFileSync(join(kids, "src", "lib.rs"), "mod inner;\npub mod mid;\npub mod outer;\n\npub use mid::{halve, made};\npub use outer::twice;\n");
  writeFileSync(join(kids, "src", "inner.rs"), "pub fn halve(n: u32) -> u32 {\n    n / 2\n}\n\npub fn made() -> u32 {\n    7\n}\n\npub fn twice(n: u32) -> u32 {\n    n * 2\n}\n");
  writeFileSync(join(kids, "src", "mid.rs"), "pub use crate::inner::{halve, made, twice};\n");
  writeFileSync(join(kids, "src", "outer.rs"), "pub use crate::mid::twice;\n");
  writeFileSync(join(kids, "index.js"), "export {halve, made} from './src/mid.js';\nexport {twice} from './src/outer.js';\n");
  writeFileSync(join(dir, "dom", "src", "by_module.rs"), "pub fn one_more() -> u32 {\n    kids::mid::made() + 1\n}\n");
  writeFileSync(join(dir, "dom", "src", "lib.rs"), "pub mod by_module;\n\npub fn run() -> u32 {\n    kids::halve(kids::twice(kids::made())) + by_module::one_more()\n}\n");
  mkdirSync(join(dir, "node_modules", "@acme"), { recursive: true });
  symlinkSync(kids, join(dir, "node_modules", "@acme", "kids"), "dir");
  mkdirSync(join(dir, "node_modules", "@rust-js"));
  symlinkSync(runtimeDir, join(dir, "node_modules", "@rust-js", "runtime"), "dir");
  const native = join(dir, "native");
  mkdirSync(native);
  run(["rustc", "--edition=2024", "--crate-type=rlib", "--crate-name", "kids", join(kids, "src", "lib.rs"), "-o", join(native, "libkids.rlib")]);
  run(["rustc", "--edition=2024", "--crate-type=rlib", "--crate-name", "dom", join(dir, "dom", "src", "lib.rs"), "-o", join(native, "libdom.rlib"), "--extern", `kids=${join(native, "libkids.rlib")}`]);
  writeFileSync(join(native, "main.rs"), "fn main() {\n    println!(\"{}\", dom::run());\n}\n");
  run(["rustc", "--edition=2024", join(native, "main.rs"), "-o", join(native, "program"), "--extern", `dom=${join(native, "libdom.rlib")}`, "-L", `dependency=${native}`]);
  const nativeRun = run([join(native, "program")]).trim();
  expect(nativeRun).toBe("15");
  const built = build(join(dir, "dom"));
  expect(built.stderr).toBe("");
  const js = readFileSync(join(dir, "dom", "src", "lib.js"), "utf8");
  // By the package's name, through the root; by its module's file, in a
  // module whose Rust names it through that module.
  expect(js).toContain('import { halve, made, twice } from "@acme/kids";\n');
  expect(js).not.toContain("inner.js");
  const byModule = readFileSync(join(dir, "dom", "src", "by_module.js"), "utf8");
  expect(byModule).toContain('import { made } from "../../kids/src/mid.js";\n');
  expect(existsSync(join(kids, "src", "lib.js"))).toBe(false);
  const ran = run([node ?? "node", "--input-type=module", "--eval", `console.log((await import(${JSON.stringify(join(dir, "dom", "src", "lib.js"))})).run());`]).trim();
  expect(ran).toBe(nativeRun);
}, 600_000);

// A workspace package's module, React's `shared/ReactFeatureFlags`, is
// imported by the package's name and its subpath, without its extension,
// where the package's `exports` resolves that subpath to the module's JS,
// as Node, Vite and jest resolve it; elsewhere by its file (the user's
// decision, ADR 0770, amended). rust-js reads `exports` and never writes
// it: what a package exports is its author's.
test("a dependency's module is imported by its package's subpath where its exports resolve it", () => {
  const dir = fixture("package-subpaths");
  writeFileSync(join(dir, "Cargo.toml"), '[workspace]\nmembers = ["shared", "dom"]\nresolver = "3"\n');
  for (const [name, more] of [["shared", ""], ["dom", '\n[dependencies]\nshared = { path = "../shared" }\n']]) {
    mkdirSync(join(dir, name, "src"), { recursive: true });
    writeFileSync(join(dir, name, "Cargo.toml"), `[package]\nname = "${name}"\nversion = "0.1.0"\nedition = "2024"\n${more}`);
  }
  const shared = join(dir, "shared");
  const manifest = (exports?: unknown) =>
    writeFileSync(join(shared, "package.json"), JSON.stringify({ name: "shared", private: true, type: "module", exports }));
  writeFileSync(join(shared, "src", "lib.rs"), "pub mod flags;\npub mod node_modules;\npub mod symbols;\n");
  writeFileSync(join(shared, "src", "node_modules.rs"), "pub fn one() -> u32 {\n    1\n}\n");
  writeFileSync(join(shared, "src", "flags.rs"), "pub const ENABLE_SCOPE_API: bool = true;\n\npub fn level() -> u32 {\n    3\n}\n");
  writeFileSync(join(shared, "src", "symbols.rs"), 'pub fn name(level: u32) -> String {\n    format!("level {level}")\n}\n');
  writeFileSync(join(dir, "dom", "src", "lib.rs"), `use shared::flags::{ENABLE_SCOPE_API, level};
use shared::node_modules::one;
use shared::symbols::name;

pub fn run() -> String {
    if ENABLE_SCOPE_API { name(level() + one()) } else { String::new() }
}
`);
  mkdirSync(join(dir, "node_modules", "@rust-js"), { recursive: true });
  symlinkSync(shared, join(dir, "node_modules", "shared"), "dir");
  symlinkSync(runtimeDir, join(dir, "node_modules", "@rust-js", "runtime"), "dir");
  symlinkSync(stdDir, join(dir, "node_modules", "@rust-js", "std"), "dir");
  const js = () => readFileSync(join(dir, "dom", "src", "lib.js"), "utf8");
  const ran = () => run([node ?? "node", "--input-type=module", "--eval", `console.log((await import(${JSON.stringify(join(dir, "dom", "src", "lib.js"))})).run());`]).trim();
  const built = () => build(join(dir, "dom"));
  const native = join(dir, "native");
  mkdirSync(native);
  run(["rustc", "--edition=2024", "--crate-type=rlib", "--crate-name", "shared", join(shared, "src", "lib.rs"), "-o", join(native, "libshared.rlib")]);
  run(["rustc", "--edition=2024", "--crate-type=rlib", "--crate-name", "dom", join(dir, "dom", "src", "lib.rs"), "-o", join(native, "libdom.rlib"), "--extern", `shared=${join(native, "libshared.rlib")}`]);
  writeFileSync(join(native, "main.rs"), "fn main() {\n    println!(\"{}\", dom::run());\n}\n");
  run(["rustc", "--edition=2024", join(native, "main.rs"), "-o", join(native, "program"), "--extern", `dom=${join(native, "libdom.rlib")}`, "-L", `dependency=${native}`]);
  const nativeRun = run([join(native, "program")]).trim();
  expect(nativeRun).toBe("level 4");
  // Each module's import, by its file or its subpath.
  const imports = (flags: string, symbols: string) => {
    for (const line of [
      `import { ENABLE_SCOPE_API, level } from "${flags}";\n`,
      'import { one } from "../../shared/src/node_modules.js";\n',
      `import { name } from "${symbols}";\n`,
    ]) {
      expect(js()).toContain(line);
    }
  };
  const [flags, symbols] = ["../../shared/src/flags.js", "../../shared/src/symbols.js"];
  // No `exports`: Node finds `shared/src/flags` by its file, which an
  // `import` names with its extension, so by its path, as before.
  manifest();
  expect(built().stderr).toBe("");
  imports(flags, symbols);
  expect(ran()).toBe(nativeRun);
  // A pattern, Node's, Vite's and jest's alike, the package rebuilt as its
  // package.json changes; a `node_modules` segment, which Node refuses in
  // a subpath, by its file.
  manifest({ "./*": "./*.js" });
  expect(built().stderr).toBe("");
  imports("shared/src/flags", "shared/src/symbols");
  expect(ran()).toBe(nativeRun);
  // Its own key, the default condition's, beside a pattern another
  // condition gives and one that hides a module, `null`.
  manifest({ "./src/flags": { import: "./src/flags.js", default: "./src/flags.js" }, "./src/*": { browser: "./src/*.js" } });
  expect(built().stderr).toBe("");
  imports("shared/src/flags", symbols);
  expect(ran()).toBe(nativeRun);
  manifest({ "./*": "./*.js", "./src/symbols": null });
  expect(built().stderr).toBe("");
  imports("shared/src/flags", symbols);
  // A subpath never moves an import, whose order is the order JS loads
  // its modules in: each is where its `use` is, and one no `use` makes, a
  // binding's of the crate's own, after them (ADR 1063).
  for (const file of ["a", "t"]) {
    writeFileSync(join(dir, `${file}.js`), `export function ${file}() {\n  return 1;\n}\n`);
  }
  const rust = readFileSync(join(dir, "dom", "src", "lib.rs"), "utf8");
  const bound = (file: string) =>
    `\n#[cfg_attr(rust_js, rust_js::link_name = "../../${file}.js#${file}")]\nfn ${file}() -> u32 {\n    unreachable!()\n}\n`;
  writeFileSync(join(dir, "dom", "src", "lib.rs"), `${rust.replace("level() + one()", "level() + one() + a() + t()")}${bound("a")}${bound("t")}`);
  const imported = () => [...js().matchAll(/^import (.*) from /gm)].map((line) => line[1]);
  manifest();
  expect(built().stderr).toBe("");
  const byFile = imported();
  manifest({ "./*": "./*.js" });
  expect(built().stderr).toBe("");
  expect(js()).toContain('from "shared/src/flags";\n');
  expect(imported()).toEqual(byFile);
  writeFileSync(join(dir, "dom", "src", "lib.rs"), rust);
  // A longer pattern's, `./src/*`'s, that keeps the extension, which Node
  // finds the file by with it, react-markup's (rd713): `node_modules.js`
  // is no `node_modules` segment.
  manifest({ "./*": "./*.js", "./src/*": "./src/*" });
  expect(built().stderr).toBe("");
  expect(js()).toContain('import { ENABLE_SCOPE_API, level } from "shared/src/flags.js";\nimport { one } from "shared/src/node_modules.js";\nimport { name } from "shared/src/symbols.js";\n');
  expect(ran()).toBe(nativeRun);
  // Exports of the root alone, and a package Node finds no copy of.
  manifest({ ".": "./src/lib.js" });
  expect(built().stderr).toBe("");
  imports(flags, symbols);
  manifest({ "./*": "./*.js" });
  rmSync(join(dir, "node_modules", "shared"));
  expect(built().stderr).toBe("");
  imports(flags, symbols);
  expect(ran()).toBe(nativeRun);
}, 600_000);

// A crate's own module whose file is in another package, React DOM's
// `#[path = "../shared/ReactDOMFragmentRefShared.rs"]`, is imported by that
// package's name and subpath, `'shared/ReactDOMFragmentRefShared'` as
// upstream writes it, where its `exports` resolves the subpath to the
// module's JS and Node finds the package from the importing module, as a
// dependency's module is (ADR 0770, amended); one in the importer's own
// package, by its file, as before; and the import stays where its `use`
// is (ADR 1063).
test("a crate's module in another package is imported by that package's subpath", () => {
  const dir = fixture("own-module-subpaths");
  mkdirSync(join(dir, "dom", "src"), { recursive: true });
  mkdirSync(join(dir, "shared"));
  writeFileSync(join(dir, "dom", "Cargo.toml"), '[package]\nname = "dom"\nversion = "0.1.0"\nedition = "2024"\n\n[workspace]\n');
  // A workspace's root package above both, as React's is: each module is
  // of the package nearest above it.
  writeFileSync(join(dir, "package.json"), JSON.stringify({ name: "workspace", private: true, type: "module" }));
  const dom = (exports?: unknown) =>
    writeFileSync(join(dir, "dom", "package.json"), JSON.stringify({ name: "dom", private: true, type: "module", exports }));
  dom();
  const manifest = (exports?: unknown) =>
    writeFileSync(join(dir, "shared", "package.json"), JSON.stringify({ name: "shared", private: true, type: "module", exports }));
  writeFileSync(join(dir, "shared", "fragment.rs"), "use crate::level::LEVEL;\n\npub struct Spot {\n    pub at: u32,\n}\n\npub fn position(n: u32) -> u32 {\n    n * LEVEL\n}\n");
  writeFileSync(join(dir, "dom", "src", "level.rs"), "pub const LEVEL: u32 = 2;\n");
  writeFileSync(join(dir, "dom", "src", "zeta.rs"), "pub fn zeta() -> u32 {\n    1\n}\n");
  writeFileSync(join(dir, "dom", "src", "lib.rs"), `#[path = "../../shared/fragment.rs"]
pub mod fragment;
pub mod level;
pub mod zeta;

use fragment::{Spot, position};
use zeta::zeta;

pub fn run() -> u32 {
    spot(3).at + zeta()
}

pub fn spot(n: u32) -> Spot {
    Spot { at: position(n) }
}

pub fn bound() -> u32 {
    a()
}

#[cfg_attr(rust_js, rust_js::link_name = "../../a.js#a")]
fn a() -> u32 {
    unreachable!()
}
`);
  writeFileSync(join(dir, "a.js"), "export function a() {\n  return 1;\n}\n");
  mkdirSync(join(dir, "node_modules", "@rust-js"), { recursive: true });
  symlinkSync(runtimeDir, join(dir, "node_modules", "@rust-js", "runtime"), "dir");
  symlinkSync(stdDir, join(dir, "node_modules", "@rust-js", "std"), "dir");
  const js = () => readFileSync(join(dir, "dom", "src", "lib.js"), "utf8");
  const imported = () => [...js().matchAll(/^import .* from "(.*)";$/gm)].map((line) => line[1]);
  const ran = () => run([node ?? "node", "--input-type=module", "--eval", `console.log((await import(${JSON.stringify(join(dir, "dom", "src", "lib.js"))})).run());`]).trim();
  const built = () => build(join(dir, "dom"));
  const native = join(dir, "native");
  mkdirSync(native);
  run(["rustc", "--edition=2024", "--crate-type=rlib", "--crate-name", "dom", join(dir, "dom", "src", "lib.rs"), "-o", join(native, "libdom.rlib")]);
  writeFileSync(join(native, "main.rs"), "fn main() {\n    println!(\"{}\", dom::run());\n}\n");
  run(["rustc", "--edition=2024", join(native, "main.rs"), "-o", join(native, "program"), "--extern", `dom=${join(native, "libdom.rlib")}`, "-L", `dependency=${native}`]);
  const nativeRun = run([join(native, "program")]).trim();
  expect(nativeRun).toBe("7");
  const byFile = ["../../shared/fragment.js", "./zeta.js", "../../a.js"];
  // No `exports`, or Node finds no copy of the package: by its file.
  manifest({ "./*": "./*.js" });
  expect(built().stderr).toBe("");
  expect(imported()).toEqual(byFile);
  manifest();
  symlinkSync(join(dir, "shared"), join(dir, "node_modules", "shared"), "dir");
  expect(built().stderr).toBe("");
  expect(imported()).toEqual(byFile);
  expect(ran()).toBe(nativeRun);
  // By its package's subpath, where its `use` put it among the imports,
  // as JS loads them in that order; its types by the same
  // specifier; the module in `shared` imports `dom`'s by its file, `dom`
  // exporting none.
  manifest({ "./*": "./*.js" });
  expect(built().stderr).toBe("");
  expect(imported()).toEqual(["shared/fragment", "./zeta.js", "../../a.js"]);
  const types = readFileSync(join(dir, "dom", "src", "lib.d.ts"), "utf8");
  expect(types).toContain('from "shared/fragment";\n');
  expect(readFileSync(join(dir, "shared", "fragment.js"), "utf8")).toContain('from "../dom/src/level.js";\n');
  expect(ran()).toBe(nativeRun);
  // `dom` exporting its modules too: the module in `shared` imports
  // `dom`'s by `dom`'s subpath, and `dom`'s own modules each other's by
  // its file, being of one package.
  dom({ "./*": "./*.js" });
  symlinkSync(join(dir, "dom"), join(dir, "node_modules", "dom"), "dir");
  expect(built().stderr).toBe("");
  expect(imported()).toEqual(["shared/fragment", "./zeta.js", "../../a.js"]);
  expect(readFileSync(join(dir, "shared", "fragment.js"), "utf8")).toContain('from "dom/src/level";\n');
  expect(ran()).toBe(nativeRun);
  // A pattern that keeps the subpath's extension, react-markup's
  // `"./src/*": "./src/*"`: by its subpath with the extension,
  // `'react-markup/src/ReactMarkupLegacyClientStreamConfig.js'` as upstream
  // writes it, from react-client's directory (rd713).
  manifest({ "./*": "./*" });
  expect(built().stderr).toBe("");
  expect(imported()).toEqual(["shared/fragment.js", "./zeta.js", "../../a.js"]);
  expect(ran()).toBe(nativeRun);
  // A pattern of another file, which Node then doesn't find: by its file.
  manifest({ "./*": "./lib/*" });
  expect(built().stderr).toBe("");
  expect(imported()).toEqual(byFile);
}, 600_000);

// A module's imports are in the order its `use`s are written, as JS loads
// its modules in its imports' order (the user's decision, ADR 1063): React
// DOM's ReactDOMFizzServerNode imports ReactFizzServer, a binding's JS
// module, before ReactFizzConfigDOM, a library's module by its package's
// subpath, whose load sets what the other's reads in jest's cycle (rd570).
// Whichever `use` group comes first, its module loads first; and a binding
// the module declares itself is imported where it's declared among them, as
// the reconciler's `#[link_name = "./ReactFiberConfig.js#.."]`s are its
// first import (rd608, ADR 1063, amended).
test("imports are in the order of the uses that make them, a binding's and a library module's alike", () => {
  const dir = fixture("use-ordered-imports");
  writeFileSync(join(dir, "Cargo.toml"), '[workspace]\nmembers = ["server", "config", "dom"]\nresolver = "3"\n');
  const crate = (name: string, more = "") => {
    mkdirSync(join(dir, name, "src"), { recursive: true });
    writeFileSync(join(dir, name, "Cargo.toml"), `[package]\nname = "${name}"\nversion = "0.1.0"\nedition = "2024"\n${more}`);
    writeFileSync(join(dir, name, "package.json"), JSON.stringify({ name, private: true, type: "module", exports: { "./*": "./*.js" } }));
  };
  crate("server");
  crate("config");
  crate("dom", '\n[dependencies]\nserver = { path = "../server" }\nconfig = { path = "../config" }\n');
  // Each JS module notes its load.
  const noted = (name: string, value: number) =>
    `globalThis.loaded = [...(globalThis.loaded ?? []), "${name}"];\n\nexport function ${name}() {\n  return ${value};\n}\n`;
  writeFileSync(join(dir, "server", "src", "Fizz.js"), noted("start", 1));
  writeFileSync(join(dir, "server", "src", "lib.rs"), "pub mod task;\n");
  writeFileSync(join(dir, "server", "src", "task.rs"), '#[cfg_attr(rust_js, rust_js::link_name = "server/src/Fizz#start")]\npub fn start() -> u32 {\n    unreachable!()\n}\n');
  writeFileSync(join(dir, "config", "src", "mark.js"), noted("mark", 2));
  writeFileSync(join(dir, "config", "src", "lib.rs"), "pub mod setup;\n");
  writeFileSync(join(dir, "config", "src", "setup.rs"), '#[cfg_attr(rust_js, rust_js::link_name = "./mark.js#mark")]\nfn mark() -> u32 {\n    unreachable!()\n}\n\npub fn make() -> u32 {\n    mark() * 2\n}\n');
  mkdirSync(join(dir, "node_modules", "@rust-js"), { recursive: true });
  for (const name of ["server", "config"]) symlinkSync(join(dir, name), join(dir, "node_modules", name), "dir");
  symlinkSync(runtimeDir, join(dir, "node_modules", "@rust-js", "runtime"), "dir");
  symlinkSync(stdDir, join(dir, "node_modules", "@rust-js", "std"), "dir");
  const lib = join(dir, "dom", "src", "lib.js");
  const imported = () => [...readFileSync(lib, "utf8").matchAll(/^import .* from "(.*)";$/gm)].map((line) => line[1]);
  const loaded = () =>
    run([node ?? "node", "--input-type=module", "--eval", `const dom = await import(${JSON.stringify(lib)}); console.log(dom.run(), globalThis.loaded.join());`]).trim();
  writeFileSync(join(dir, "own.js"), noted("own", 3));
  const own = '#[cfg_attr(rust_js, rust_js::link_name = "../../own.js#own")]\nfn own() -> u32 {\n    unreachable!()\n}';
  const written = (first: string, second: string, between = "") => {
    const declared = between ? `${between}\n\n` : "";
    const ran = between ? "start() + make() + own()" : "start() + make()";
    writeFileSync(join(dir, "dom", "src", "lib.rs"), `${first}\n\n${declared}${second}\n\npub fn run() -> u32 {\n    ${ran}\n}\n`);
    expect(build(join(dir, "dom")).stderr).toBe("");
  };
  // ReactDOMFizzServerNode's order: the binding's, then the library's.
  written("use server::task::start;", "use config::setup::make;");
  expect(imported()).toEqual(["server/src/Fizz", "config/src/setup"]);
  expect(loaded()).toBe("5 start,mark");
  // The other way round, the library's module first, which a sort by the
  // specifier, or by its file, would put after the binding's.
  written("use config::setup::make;", "use server::task::start;");
  expect(imported()).toEqual(["config/src/setup", "server/src/Fizz"]);
  expect(loaded()).toBe("5 mark,start");
  // A binding of its own declared between the two, loaded between them.
  written("use config::setup::make;", "use server::task::start;", own);
  expect(imported()).toEqual(["config/src/setup", "../../own.js", "server/src/Fizz"]);
  expect(loaded()).toBe("8 mark,own,start");
}, 600_000);
