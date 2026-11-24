// The tooling net's tree shaking (Tu's promise: the JS rust-js writes plays
// with the tools a frontend uses). A bundler keeps only what an app uses
// of a rust-js library: an app importing one of a library's exports bundles
// that export and what it reads, none of the others' functions, the
// dictionaries, classes, namespaces and constants they alone read, nor
// anything the library makes as its module loads. An app importing the
// library and using none of it bundles nothing of it. Each bundler a
// frontend builds with is asked, minified as a production build is:
// Vite's (Rolldown, oxc's minifier) and Bun's. Nightly (TESTS.md).
import { beforeAll, expect, test } from "bun:test";
import { writeFileSync } from "node:fs";
import { join } from "node:path";
import { compileJsLibrary } from "./programs";
import { buildCompiler, compiler, outsideFixture, run } from "./support";
import { underLoad } from "../scripts/timing";

beforeAll(buildCompiler, 600_000);

// A library of the kinds of item rust-js writes at a module's top: plain
// functions, one that calls another, a struct and its derived traits, a
// trait and its impls' dictionaries, a namespace's methods, an enum with
// fields and its `Display`, a type no other crate can make, a `const`, a
// `static` and a thread-local. A class is apart, below.
const library = `use std::cell::Cell;
use std::fmt;

pub fn double(n: u32) -> u32 {
    n * 2
}

pub fn quadruple(n: u32) -> u32 {
    double(double(n))
}

#[derive(Debug, Clone, PartialEq)]
pub struct Order {
    pub id: u32,
    pub items: Vec<String>,
}

pub fn order_total(order: &Order) -> usize {
    order.items.iter().map(|item| item.len()).sum()
}

pub trait Shape {
    fn area(&self) -> f64;
}

pub struct Circle(pub f64);
pub struct Square(pub f64);

impl Shape for Circle {
    fn area(&self) -> f64 {
        3.14159 * self.0 * self.0
    }
}

impl Shape for Square {
    fn area(&self) -> f64 {
        self.0 * self.0
    }
}

pub fn shapes_area(shapes: &[Box<dyn Shape>]) -> f64 {
    shapes.iter().map(|shape| shape.area()).sum()
}

pub enum Event {
    Click { x: i32, y: i32 },
    Key(String),
    Idle,
}

impl fmt::Display for Event {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        match self {
            Event::Click { x, y } => write!(f, "click at {x},{y}"),
            Event::Key(key) => write!(f, "key {key}"),
            Event::Idle => write!(f, "idle"),
        }
    }
}

pub fn describe(event: &Event) -> String {
    event.to_string()
}

pub struct Email(String);

impl Email {
    pub fn parse(text: &str) -> Option<Email> {
        text.contains('@').then(|| Email(text.to_string()))
    }

    pub fn domain(&self) -> &str {
        self.0.split('@').nth(1).unwrap_or("")
    }
}

pub const GREETING: &str = "a greeting none of the double app reads";

pub static LIMITS: [u32; 3] = [10, 20, 30];

pub fn limit(at: usize) -> u32 {
    LIMITS[at]
}

thread_local! {
    static SEEN: Cell<u32> = const { Cell::new(0) };
}

pub fn seen() -> u32 {
    SEEN.with(|seen| {
        seen.set(seen.get() + 1);
        seen.get()
    })
}
`;

/** Each export of the library, the words in its JS that only it has: none
 * of which a bundle of an app that doesn't use it may hold. */
const marks: Record<string, string[]> = {
  quadruple: ["quadruple"],
  orderTotal: ["orderTotal"],
  shapesArea: ["shapesArea"],
  describe: ["click at", "key "],
  Email: ["domain"],
  GREETING: ["a greeting none of the double app reads"],
  limit: ["limit"],
  seen: ["seen"],
};

/** The library compiled into `dir/lib`, as a package of its own. */
function compiled(dir: string): void {
  writeFileSync(join(dir, "lib.rs"), library);
  const built = compileJsLibrary(join(dir, "lib.rs"), dir, "2024", "lib");
  if ("error" in built) throw new Error(built.error);
  writeFileSync(join(dir, "lib", "package.json"), JSON.stringify({ name: "lib", type: "module", sideEffects: false, exports: { ".": "./lib.js" } }));
}

/** What Vite, Rolldown minified by oxc as its production build is, bundles
 * of `entry`. */
async function viteBundle(dir: string, entry: string): Promise<string> {
  const { build } = await import("vite");
  const out = await build({
    root: dir,
    logLevel: "silent",
    configFile: false,
    build: { write: false, minify: true, lib: { entry: join(dir, entry), formats: ["es"], fileName: "bundle" } },
  });
  const code = ((Array.isArray(out) ? out[0] : out) as { output: { code: string }[] }).output[0]!.code;
  // Without the region comments naming each module's file, which two
  // bundles of one code differ by.
  return code.replace(/^\/\/#(?:end)?region.*\n?/gm, "");
}

/** What Bun's bundler, minified, bundles of `entry`. */
async function bunBundle(dir: string, entry: string): Promise<string> {
  const out = await Bun.build({ entrypoints: [join(dir, entry)], minify: true, format: "esm", target: "browser" });
  if (!out.success) throw new Error(out.logs.join("\n"));
  return out.outputs[0]!.text();
}

/** `source` compiled into `dir/<crate>`, as a package of its own. */
function compiledAs(dir: string, crate: string, source: string): void {
  writeFileSync(join(dir, `${crate}.rs`), source);
  const built = compileJsLibrary(join(dir, `${crate}.rs`), dir, "2024", crate);
  if ("error" in built) throw new Error(built.error);
  writeFileSync(join(dir, crate, "package.json"), JSON.stringify({ name: crate, type: "module", sideEffects: false, exports: { ".": "./lib.js" } }));
}

const bundlers = { vite: viteBundle, bun: bunBundle };

for (const [name, bundle] of Object.entries(bundlers)) {
  // The bundle of what the app uses is the same with the library's other
  // exports as without them: the library of `double` alone, compiled apart.
  test(`${name}: an app that imports one of a library's exports bundles none of the others`, async () => {
    const dir = outsideFixture(`tree-shaking-one-${name}`);
    compiled(dir);
    compiledAs(dir, "alone", "pub fn double(n: u32) -> u32 {\n    n * 2\n}\n");
    writeFileSync(join(dir, "entry.js"), `import { double } from "./lib/lib.js";\nglobalThis.out = double(21);\n`);
    writeFileSync(join(dir, "alone.js"), `import { double } from "./alone/lib.js";\nglobalThis.out = double(21);\n`);
    const [code, alone] = [await bundle(dir, "entry.js"), await bundle(dir, "alone.js")];
    const kept = Object.entries(marks).filter(([, words]) => words.some((word) => code.includes(word))).map(([item]) => item);
    expect(kept).toEqual([]);
    expect(code.length).toBe(alone.length);
  }, underLoad(120_000));

  test(`${name}: an app that imports a library and uses none of it bundles nothing of it`, async () => {
    const dir = outsideFixture(`tree-shaking-none-${name}`);
    compiled(dir);
    writeFileSync(join(dir, "entry.js"), `import "./lib/lib.js";\nglobalThis.out = 1;\n`);
    writeFileSync(join(dir, "bare.js"), `globalThis.out = 1;\n`);
    expect(await bundle(dir, "entry.js")).toBe(await bundle(dir, "bare.js"));
  }, underLoad(120_000));

  // A struct's checker is its table's constant, marked pure (#2403, the
  // lead's (B)): of an export the app doesn't import, dropped; its field's
  // key, which no minifier renames, shows it. Of one it does, kept.
  test(`${name}: an unused export's checker is dropped`, async () => {
    const dir = outsideFixture(`tree-shaking-checker-${name}`);
    compiledAs(dir, "tickets", "pub struct Ticket {\n    pub seat_number_marker: u32,\n}\n\npub fn book(ticket: &Ticket) -> u32 {\n    ticket.seat_number_marker\n}\n\npub fn double(n: u32) -> u32 {\n    n * 2\n}\n");
    writeFileSync(join(dir, "entry.js"), `import { double } from "./tickets/lib.js";\nglobalThis.out = double(21);\n`);
    writeFileSync(join(dir, "booked.js"), `import { book } from "./tickets/lib.js";\nglobalThis.out = book({ seatNumberMarker: 1 });\n`);
    expect(await bundle(dir, "entry.js")).not.toContain("seatNumberMarker");
    expect(await bundle(dir, "booked.js")).toContain("seatNumberMarker");
  }, underLoad(120_000));

  // A check's message names its function and parameter by strings, which
  // a minifier keeps, so a minified bundle's failure reads as the
  // unminified one's (#2403, ADR 1192, amended).
  test(`${name}: a check's message survives minification`, async () => {
    const dir = outsideFixture(`tree-shaking-message-${name}`);
    compiledAs(dir, "checked", "pub fn double(n: u32) -> u32 {\n    n * 2\n}\n");
    writeFileSync(
      join(dir, "entry.js"),
      `import { double } from "./checked/lib.js";\ntry { double(-1); } catch (error) { globalThis.out = String(error); }\n`,
    );
    writeFileSync(join(dir, "bundle.js"), await bundle(dir, "entry.js"));
    await import(join(dir, "bundle.js"));
    expect((globalThis as { out?: string }).out).toBe("RangeError: rust-js: double(n): expected a u32, got -1");
  }, underLoad(120_000));

  // A type no other crate can make is branded where Rust makes it; the
  // registry of brands, a module's top's, is dropped where nothing the app
  // uses brands or checks one: its key, which no minifier renames, shows it.
  test(`${name}: an unused export's brands are dropped`, async () => {
    const dir = outsideFixture(`tree-shaking-brands-${name}`);
    compiledAs(dir, "sealed", "pub struct Email(String);\n\npub fn make(text: &str) -> Email {\n    Email(text.to_string())\n}\n\npub fn domain(email: &Email) -> usize {\n    email.0.len()\n}\n\npub fn double(n: u32) -> u32 {\n    n * 2\n}\n");
    writeFileSync(join(dir, "entry.js"), `import { double } from "./sealed/lib.js";\nglobalThis.out = double(21);\n`);
    writeFileSync(join(dir, "branded.js"), `import { domain, make } from "./sealed/lib.js";\nglobalThis.out = domain(make("a@b"));\n`);
    const key = "the brand sets of the types no other crate can make";
    expect(await bundle(dir, "entry.js")).not.toContain(key);
    expect(await bundle(dir, "branded.js")).toContain(key);
  }, underLoad(120_000));

  // A library's module of functions its own JS doesn't check, which an app
  // re-exports as a namespace: a module of the app's in its place, `export *
  // from` it and a checked forwarder of each, its own export shadowing the
  // star's (#2536). Bundled, a member the app reads is the checked one, and
  // one it doesn't is dropped, its body's text gone.
  test(`${name}: a namespace of a library's unchecked functions keeps its checks and drops what's unused`, async () => {
    const dir = outsideFixture(`tree-shaking-namespace-${name}`);
    compiledAs(dir, "dep", "pub mod m {\n    pub fn used(n: u32) -> u32 {\n        n + 1\n    }\n\n    pub fn unused_member_marker(n: u32) -> u32 {\n        n * 7919\n    }\n}\n");
    writeFileSync(join(dir, "app.rs"), "pub use dep::m as ns;\n\npub fn one() -> u32 {\n    ns::used(0)\n}\n");
    run([compiler, join(dir, "app.rs"), "-o", join(dir, "app", "lib.js"), "--dependency", join(dir, "dep", "lib.manifest.json"),
      "--", "--crate-name", "app", "--edition=2024", "--extern", `dep=${join(dir, "libdep.rmeta")}`, "-L", dir]);
    writeFileSync(join(dir, "entry.js"), `import { ns } from "./app/lib.js";\ntry { ns.used(-1); } catch (error) { globalThis.out = String(error); }\n`);
    const code = await bundle(dir, "entry.js");
    expect(code).not.toContain("7919");
    writeFileSync(join(dir, "bundle.js"), code);
    await import(join(dir, "bundle.js"));
    expect((globalThis as { out?: string }).out).toBe("RangeError: rust-js: used(n): expected a u32, got -1");
  }, underLoad(120_000));

  // A class is an ES class, its members in its body: a declaration a
  // bundler drops where nothing uses it (#2495).
  test(`${name}: an unused class is dropped`, async () => {
    const dir = outsideFixture(`tree-shaking-class-${name}`);
    compiledAs(dir, "classes", `${"#[cfg_attr(rust_js, rust_js::class)]\npub struct Counter {\n    pub count: u32,\n}\n\n#[cfg_attr(rust_js, rust_js::class)]\nimpl Counter {\n    pub fn new() -> Self {\n        Counter { count: 0 }\n    }\n\n    pub fn bump(&mut self) -> u32 {\n        self.count += 1;\n        self.count\n    }\n}\n\n"}pub fn double(n: u32) -> u32 {\n    n * 2\n}\n`);
    // The module used, for `double`: a package's `sideEffects: false` drops
    // only a module nothing of which is used.
    writeFileSync(join(dir, "entry.js"), `import { double } from "./classes/lib.js";\nglobalThis.out = double(21);\n`);
    expect(await bundle(dir, "entry.js")).not.toContain("bump");
  }, underLoad(120_000));
}
