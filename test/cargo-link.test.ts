import { beforeAll, expect, test } from "bun:test";
import { mkdirSync, readFileSync, writeFileSync } from "node:fs";
import { join } from "node:path";
import { runSync } from "./child";
import { buildReact, compiler, fixture, root, run, target } from "./support";
import { parseManifest } from "../tooling/manifest.js";
import { channel } from "../scripts/toolchain.ts";

beforeAll(buildReact, 600_000);

// A library is compiled by rust-js, which writes its JS, its manifest, and
// the metadata its consumers' rustc reads, from one build (ADR 0100).
test("a real Cargo dependency links functions and function values into JS", async () => {
  const dir = fixture("cargo-shared-library");
  writeFileSync(join(dir, "Cargo.toml"), '[workspace]\nmembers = ["shared", "app"]\nresolver = "2"\n');
  for (const name of ["shared", "app"]) {
    mkdirSync(join(dir, name, "src"), { recursive: true });
    writeFileSync(join(dir, name, "Cargo.toml"), `[package]\nname = "${name}"\nversion = "0.1.0"\nedition = "2024"\n` + (name === "app" ? '[dependencies]\nmodel = { package = "shared", path = "../shared" }\n' : ''));
  }
  const source = join(dir, "shared/src/lib.rs");
  writeFileSync(source, 'pub fn increment(n: u32) -> u32 { n + 1 }\npub fn positive(n: i32) -> bool { n > 0 }\n');
  writeFileSync(join(dir, "app/src/lib.rs"), 'pub mod nested { pub mod inner { pub fn run() -> u32 { let increment = model::increment; if model::positive(1) { increment(40) + 1 } else { 0 } } } }\npub fn run() -> u32 { nested::inner::run() }\n');
  writeFileSync(join(dir, "app/src/main.rs"), 'fn main() { println!("{}", app::run()); }');
  const manifestPath = join(dir, "app/Cargo.toml");
  run(["cargo", `+${channel}`, "generate-lockfile", "--offline", "--manifest-path", manifestPath]);
  const libraryManifest = join(dir, "js/shared/manifest.json");
  const metadata = join(dir, "js/shared/libshared.rmeta");
  const output = join(dir, "js/app/lib.js"), manifest = join(dir, "js/app/manifest.json");
  const library = () => run([compiler, source, "-o", join(dir, "js/shared/lib.js"), "--library", "--manifest", libraryManifest, "--", "--crate-name", "shared", `--emit=metadata=${metadata}`]);
  // Cargo's alias for it, `model`, doesn't change the crate it is.
  const command = [compiler, join(dir, "app/src/lib.rs"), "-o", output, "--manifest", manifest, "--dependency", libraryManifest, "--", "--crate-name", "app", "--extern", `model=${metadata}`];
  library();
  run(command);
  const native = () => Number(run(["cargo", `+${channel}`, "run", "--frozen", "--quiet", "--manifest-path", manifestPath]));
  const javascript = () => Number(run([process.execPath, "--eval", `const m = await import(${JSON.stringify(output)}); console.log(m.run());`]));
  expect(javascript()).toBe(native());
  expect(javascript()).toBe(42);
  const generated = readFileSync(join(dir, "js/app/nested/inner.js"), "utf8");
  expect(generated).toContain('../../shared/lib.js');
  const consumer = parseManifest(readFileSync(manifest, "utf8"));
  expect(consumer.sources).toContain(libraryManifest);
  expect(consumer.sources).toContain(source);
  writeFileSync(source, 'pub fn increment(n: u32) -> u32 { n + 2 }\npub fn positive(n: i32) -> bool { n > 0 }\n');
  expect(() => run(command)).toThrow("dependency artifact changed");
  library();
  run(command);
  expect(javascript()).toBe(native());
  expect(javascript()).toBe(43);
  const previous = readFileSync(output, "utf8");
  const original = readFileSync(libraryManifest, "utf8");
  parseManifest(original);
  for (const [edit, message] of [
    [(m: any) => m.library.version = 2, "unsupported library ABI 2; expected 4"],
    [(m: any) => m.compiler.toolchain = "wrong", "incompatible dependency compiler identity"],
    [(m: any) => m.library.items = [], "isn't one of the items `shared`'s manifest exports"],
  ] as const) {
    const contract = JSON.parse(original);
    edit(contract);
    writeFileSync(libraryManifest, JSON.stringify(contract));
    expect(() => run(command)).toThrow(message);
    expect(readFileSync(output, "utf8")).toBe(previous);
  }
  writeFileSync(libraryManifest, original);
  writeFileSync(join(dir, "js/shared/lib.js"), "export function increment() { return 999; }");
  expect(() => run(command)).toThrow("dependency artifact changed");
  expect(readFileSync(output, "utf8")).toBe(previous);
}, 120_000);

test("a library's generic and aggregate functions link too, from a sibling output", () => {
  const dir = fixture("sibling-library");
  const source = join(dir, "shared.rs"), metadata = join(dir, "libshared.rmeta");
  writeFileSync(source, `
    pub fn notify() {}
    pub fn negate(value: bool) -> bool { !value }
    pub fn generic<T>(value: T) -> T { value }
    pub fn pair() -> (u32, u32) { (1, 2) }
  `);
  const libraryManifest = join(dir, "shared.json");
  run([compiler, source, "-o", join(dir, ".shared.js"), "--library", "--manifest", libraryManifest, "--", "--crate-name", "shared", `--emit=metadata=${metadata}`]);
  const app = join(dir, "app.rs"), output = join(dir, "app.js");
  const command = [compiler, app, "-o", output, "--dependency", libraryManifest, "--", "--extern", `shared=${metadata}`];
  const result = () => run([process.execPath, "--eval", `const m = await import(${JSON.stringify(output)}); console.log(m.run());`]).trim();
  writeFileSync(app, "pub fn run() -> bool { shared::notify(); shared::negate(false) }");
  run(command);
  expect(readFileSync(output, "utf8")).toContain('from "./.shared.js"');
  expect(result()).toBe("true");
  writeFileSync(app, "pub fn run() -> u32 { shared::generic(1u32) + shared::pair().1 }");
  run(command);
  expect(result()).toBe("3");
}, 120_000);

// ADR 0626: a library gives a crate a function holding a value with a
// destructor only through its own types: a closure or `impl Fn` it makes,
// or a `dyn Fn` its exports reach. Of one whose items the crate uses are
// plain, a `bool`'s, the crate's `dyn Fn`s drop only as its own closures
// say: React's scheduler, of `shared`'s `__EXPERIMENTAL__`, moves an
// `Rc<Task>` of a `Box<dyn FnMut>` into `with_borrow_mut`. Of one that hands
// it an `impl FnMut` of its own, they may drop.
test("a library gives a crate's dyn Fns a drop only through its own functions", () => {
  const dir = fixture("cargo-library-functions");
  const library = (name: string, source: string) => {
    mkdirSync(join(dir, name), { recursive: true });
    const manifest = join(dir, name, "manifest.json"), metadata = join(dir, name, `lib${name}.rmeta`);
    writeFileSync(join(dir, name, "lib.rs"), source);
    run([compiler, join(dir, name, "lib.rs"), "-o", join(dir, name, "lib.js"), "--library", "--manifest", manifest, "--", "--crate-name", name, `--emit=metadata=${metadata}`]);
    return { manifest, metadata };
  };
  const plain = library("plain", "pub fn flag() -> bool {\n    true\n}\n");
  const giving = library("giving", `pub struct Loud;

impl Drop for Loud {
    fn drop(&mut self) {}
}

pub fn make() -> impl FnMut(bool) + 'static {
    let held = Loud;
    move |_| {
        let _ = &held;
    }
}
`);
  const boxing = library("boxing", `pub struct Loud;

impl Drop for Loud {
    fn drop(&mut self) {}
}

pub fn boxed() -> Box<dyn FnMut(bool)> {
    let held = Loud;
    Box::new(move |_| {
        let _ = &held;
    })
}
`);
  const app = (name: string, from: { manifest: string; metadata: string }, made: string) => {
    mkdirSync(join(dir, name), { recursive: true });
    writeFileSync(join(dir, name, "lib.rs"), `use std::cell::RefCell;
use std::rc::Rc;

pub struct Task {
    pub callback: RefCell<Option<Box<dyn FnMut(bool)>>>,
}

thread_local! {
    static TASK_QUEUE: RefCell<Vec<Rc<Task>>> = const { RefCell::new(Vec::new()) };
}

pub fn queued(callback: Box<dyn FnMut(bool)>) -> usize {
    let task = Rc::new(Task {
        callback: RefCell::new(Some(callback)),
    });
    TASK_QUEUE.with_borrow_mut(|queue| queue.push(task));
    TASK_QUEUE.with_borrow(Vec::len)
}

fn boxed<F: FnMut(bool) + 'static>(f: F) -> Box<dyn FnMut(bool)> {
    Box::new(f)
}

pub fn run() -> usize {
    queued(${made})
}
`);
    return runSync([compiler, join(dir, name, "lib.rs"), "-o", join(dir, name, "app.js"), "--dependency", from.manifest, "--", "--crate-name", name, "--extern", `${name.replace("_app", "")}=${from.metadata}`], root, 120_000);
  };
  const ok = app("plain_app", plain, "boxed(|_| {\n        let _ = plain::flag();\n    })");
  expect([ok.code, ok.stderr]).toEqual([0, ""]);
  // Its `impl FnMut`, made a `dyn` through a type parameter.
  const refused = app("giving_app", giving, "boxed(giving::make())");
  expect([refused.code === 0, refused.stderr]).toEqual([false, expect.stringContaining("with_borrow_mut` of a value with a destructor")]);
  // A `dyn Fn` its export gives, made of one of its own closures.
  const given = app("boxing_app", boxing, "boxing::boxed()");
  expect([given.code === 0, given.stderr]).toEqual([false, expect.stringContaining("with_borrow_mut` of a value with a destructor")]);
});


// ADR 0633: a consumer that names a binding through a library's module
// that holds it reads it through that module, as react-reconciler reads
// `shared/assign` and `shared/ReactSharedInternals`: its manifest says what
// each module exports of a binding, which the consumer imports.
test("a library's module that holds a binding is what a consumer reads it through", async () => {
  const dir = fixture("held-binding-library");
  const extern = ["--extern", `js=${join(target, "libjs.rmeta")}`, "-L", target];
  mkdirSync(join(dir, "shared"), { recursive: true });
  writeFileSync(join(dir, "shared/lib.rs"), `pub mod assign;
pub mod internals;

pub struct Internals(core::marker::PhantomData<js::JsObject>);

unsafe extern "Rust" {
    #[cfg_attr(rust_js, rust_js::name = "ReactSharedInternals")]
    #[link_name = "react#*.__CLIENT_INTERNALS_DO_NOT_USE_OR_WARN_USERS_THEY_CANNOT_UPGRADE"]
    pub safe static REACT_SHARED_INTERNALS: &'static Internals;
}
`);
  writeFileSync(join(dir, "shared/assign.rs"), "pub use js::object::assign;\n\njs::export_default!(assign);\n");
  writeFileSync(join(dir, "shared/internals.rs"), "pub use crate::REACT_SHARED_INTERNALS;\n\njs::export_default!(REACT_SHARED_INTERNALS);\n");
  const libraryManifest = join(dir, "shared/manifest.json"), metadata = join(dir, "shared/libshared.rmeta");
  run([compiler, join(dir, "shared/lib.rs"), "-o", join(dir, "shared/lib.js"), "--library", "--manifest", libraryManifest, "--", "--crate-name", "shared", `--emit=metadata=${metadata}`, ...extern]);
  const contract = JSON.parse(readFileSync(libraryManifest, "utf8")).library;
  expect(contract.held.map((held: any) => [held.module, held.rust_name, held.export])).toEqual([
    [["assign"], "assign", "default"],
    [["internals"], "REACT_SHARED_INTERNALS", "default"],
  ]);
  mkdirSync(join(dir, "app"), { recursive: true });
  writeFileSync(join(dir, "app/lib.rs"), `use js::Unknown;
use shared::assign::assign;

pub fn merged<'a>(target: &'a Unknown, source: &Unknown) -> &'a Unknown {
    assign(target, (source,))
}

pub fn internals() -> &'static shared::Internals {
    shared::internals::REACT_SHARED_INTERNALS
}

pub fn direct<'a>(target: &'a Unknown, source: &Unknown) -> &'a Unknown {
    js::object::assign(target, (source,))
}
`);
  const output = join(dir, "app/lib.js");
  run([compiler, join(dir, "app/lib.rs"), "-o", output, "--dependency", libraryManifest, "--", "--crate-name", "app", "--extern", `shared=${metadata}`, ...extern]);
  const app = readFileSync(output, "utf8");
  expect(app).toContain('import assign from "../shared/assign.js";\nimport ReactSharedInternals from "../shared/internals.js";\n');
  expect(app).toContain("  return assign(target, source);\n");
  expect(app).toContain("  return ReactSharedInternals;\n");
  expect(app).toContain("  return Object.assign(target, source);\n");
  expect(app).not.toContain('"react"');
  mkdirSync(join(dir, "node_modules/react"), { recursive: true });
  writeFileSync(join(dir, "node_modules/react/package.json"), '{ "name": "react", "type": "module", "main": "index.js" }\n');
  writeFileSync(join(dir, "node_modules/react/index.js"), "export const __CLIENT_INTERNALS_DO_NOT_USE_OR_WARN_USERS_THEY_CANNOT_UPGRADE = { H: null };\n");
  const lib = await import(output);
  expect(lib.merged({ a: 1 }, { b: 2 })).toEqual({ a: 1, b: 2 });
  const react = await import(join(dir, "node_modules/react/index.js"));
  expect(lib.internals()).toBe(react.__CLIENT_INTERNALS_DO_NOT_USE_OR_WARN_USERS_THEY_CANNOT_UPGRADE);
}, 120_000);
