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
import { buildCompiler, outsideFixture } from "./support";
import { underLoad } from "../scripts/timing";

beforeAll(buildCompiler, 600_000);

// A library of the kinds of item rust-js writes at a module's top: plain
// functions, one that calls another, a struct and its derived traits, a
// trait and its impls' dictionaries, a namespace's methods, an enum with
// fields and its `Display`, a type no other crate can make, a `const`, a
// `static` and a thread-local. A class is apart, below (#2495).
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

  // A class's methods are assigned to its prototype at its module's top, a
  // side effect a bundler keeps whatever uses it (#2495): marked failing
  // until it's fixed, when this fails and the mark comes off.
  test.failing(`${name}: an unused class is dropped (#2495)`, async () => {
    const dir = outsideFixture(`tree-shaking-class-${name}`);
    compiledAs(dir, "classes", `${"#[cfg_attr(rust_js, rust_js::class)]\npub struct Counter {\n    pub count: u32,\n}\n\n#[cfg_attr(rust_js, rust_js::class)]\nimpl Counter {\n    pub fn new() -> Self {\n        Counter { count: 0 }\n    }\n\n    pub fn bump(&mut self) -> u32 {\n        self.count += 1;\n        self.count\n    }\n}\n\n"}pub fn double(n: u32) -> u32 {\n    n * 2\n}\n`);
    // The module used, for `double`: a package's `sideEffects: false` drops
    // only a module nothing of which is used.
    writeFileSync(join(dir, "entry.js"), `import { double } from "./classes/lib.js";\nglobalThis.out = double(21);\n`);
    expect(await bundle(dir, "entry.js")).not.toContain("bump");
  }, underLoad(120_000));
}
