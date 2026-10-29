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
import { buildCompiler, compiler, fixture, root, run } from "./support";

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
  symlinkSync(join(root, "runtime"), join(dir, "node_modules", "@rust-js", "runtime"));
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
    symlinkSync(join(root, "runtime"), join(dir, "node_modules", "@rust-js", "runtime"), "dir");
    symlinkSync(join(root, "std"), join(dir, "node_modules", "@rust-js", "std"), "dir");
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
  symlinkSync(join(root, "runtime"), join(dir, "node_modules", "@rust-js", "runtime"), "dir");
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
