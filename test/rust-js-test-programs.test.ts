// `rust-js test` of a program (ADR 1076, amended, #597): its library is
// compiled as `rust-js build` compiles it, so its tests run the JS it
// ships, and its integration tests, crates of their own that use it, are
// held to what it decided as a program, each refused where it would break
// one.

import { beforeAll, expect, test } from "bun:test";
import { existsSync, readFileSync } from "node:fs";
import { join } from "node:path";
import { buildCompiler, compiler, root } from "./support";
import { counts, rustJsTest, testCrate } from "./rust-js-tests";

beforeAll(buildCompiler, 600_000);

// A function boxed into a `dyn Fn` of an `Rc`'s struct, as alien-signals'
// `effect` boxes it: a library, whose consumers may give it what drops,
// counts the `Rc` and drops the box; the program, whose callers give it
// nothing that drops, does neither.
const lib = `use std::cell::RefCell;
use std::rc::Rc;

pub trait Shape {
    const SIDES: u32;
    const ANGLES: u32;
}

pub struct Square;

impl Shape for Square {
    const SIDES: u32 = 4;
    const ANGLES: u32 = 4;
}

pub fn angles<T: Shape>() -> u32 {
    T::ANGLES
}

pub trait Named {
    fn name(&self) -> String;
}

impl Named for Square {
    fn name(&self) -> String {
        "square".into()
    }
}

pub fn count(shapes: Vec<Box<dyn Named>>) -> usize {
    shapes.len()
}

pub struct Node {
    pub run: Box<dyn Fn() -> u32>,
}

pub fn node(run: impl Fn() -> u32 + 'static) -> Rc<Node> {
    Rc::new(Node { run: Box::new(run) })
}

pub fn run(node: &Rc<Node>) -> u32 {
    (node.run)()
}

pub struct Counter {
    pub n: RefCell<u32>,
}

pub fn counter() -> Counter {
    Counter { n: RefCell::new(0) }
}

pub fn bump(counter: &Counter) {
    *counter.n.borrow_mut() += 1;
}
`;

const loud = `use std::cell::Cell;
use std::rc::Rc;

struct Loud(Rc<Cell<u32>>);

impl Drop for Loud {
    fn drop(&mut self) {
        self.0.set(self.0.get() + 1);
    }
}
`;

const units = (dir: string) => join(dir, "node_modules/.cache/rust-js/test/wasm32-unknown-unknown/debug/deps/rust-js");

test("a test build compiles the package's library as rust-js build does, and its integration tests run that JS", () => {
  const dir = testCrate("rust-js-test-program", {
    "src/lib.rs": lib,
    "tests/nodes.rs": `use std::rc::Rc;

use app::{Shape, Square};

fn angles<T: Shape>() -> u32 {
    T::ANGLES
}

#[test]
fn a_constant_the_programs_generic_code_reads_is_read_of_a_type_parameter() {
    assert_eq!(angles::<Square>(), app::angles::<Square>());
}

#[test]
fn a_node_runs_its_function() {
    let shared = Rc::new(3);
    let node = app::node(move || *shared + 1);
    assert_eq!(app::run(&node), 4);
    assert_eq!(app::count(vec![Box::new(app::Square)]), 1);
}
`,
  });
  const { code, said } = rustJsTest(dir);
  expect([counts("node", said), code], said).toEqual([{ pass: 2, fail: 0, skip: 0 }, 0]);
  const built = Bun.spawnSync(["bun", join(root, "tooling", "cli.js"), "build"], { cwd: dir, env: { ...process.env, RUST_JS_COMPILER: compiler } });
  expect(built.exitCode, built.stderr.toString()).toBe(0);
  // The library its integration test imports: `rust-js build`'s, byte for
  // byte, the `Rc` the value itself, `{ run }`, no count.
  const imported = /from "\.\.\/(app-[0-9a-f]+)\/lib\.js"/.exec(readFileSync(join(units(dir), [...new Bun.Glob("nodes-*/nodes.js").scanSync({ cwd: units(dir) })][0]), "utf8"))?.[1];
  const tested = readFileSync(join(units(dir), imported ?? "", "lib.js"), "utf8");
  expect(tested).toBe(readFileSync(join(dir, "src/lib.js"), "utf8"));
  expect(tested).toContain("return { run };");
  expect(tested).not.toContain("strong");
  // Its contract, which its integration tests read: written by a test
  // build alone.
  const manifest = JSON.parse(readFileSync(join(units(dir), imported ?? "", "lib.manifest.json"), "utf8"));
  // The constant its own generic code reads, which its dictionaries carry.
  expect(manifest.library.dictionary_consts).toHaveLength(1);
  expect(manifest.library.dyns_drop_nothing).toHaveLength(1);
  expect(existsSync(join(dir, "src/lib.manifest.json"))).toBe(false);
});

// Each decision the program made as one, which its integration test would
// break, refused, as native Rust's answer differs from the JS's.
test.each([
  [
    "reads a trait's constant of a type parameter, which the program's dictionaries carry only where its own generic code reads it",
    `use app::{Shape, Square};

fn sides<T: Shape>() -> u32 {
    T::SIDES
}

#[test]
fn four() {
    assert_eq!(sides::<Square>(), 4);
}
`,
    "this crate reads `app::Shape::SIDES` of a type parameter, which may be `app`'s `app::Square`, but `app` is a program compiled as `rust-js build` compiles it, whose dictionary of `app::Square` carries only the constants its own generic code reads, and none reads it (ADR 1076): read it of `app::Square` itself, `<app::Square as app::Shape>::SIDES`",
  ],
  [
    "makes a dyn of the program's trait of what drops, where none of the program's own implementors drops",
    `${loud}
impl app::Named for Loud {
    fn name(&self) -> String {
        "loud".into()
    }
}

#[test]
fn dropped() {
    let drops = Rc::new(Cell::new(0));
    assert_eq!(app::count(vec![Box::new(Loud(Rc::clone(&drops)))]), 1);
    assert_eq!(drops.get(), 1);
}
`,
    "this `dyn app::Named` holds a `Loud`, whose drop runs code, but `app`, a program compiled as `rust-js build` compiles it, drops nothing of a `dyn app::Named`, as none of its own implementors drops (ADR 1076)",
  ],
  [
    "counts an Rc the program counts none of",
    `use std::rc::Rc;

#[test]
fn two() {
    let node = app::node(|| 1);
    assert_eq!(Rc::strong_count(&Rc::clone(&node)), 2);
}
`,
    "this crate counts an `Rc<app::Node>`",
  ],
  [
    "gives the program's function a closure holding what drops, which the program boxes as one that drops nothing",
    `${loud}
#[test]
fn dropped() {
    let drops = Rc::new(Cell::new(0));
    let loud = Loud(Rc::clone(&drops));
    drop(app::node(move || loud.0.get()));
    assert_eq!(drops.get(), 1);
}
`,
    "a closure that holds a value with a destructor",
  ],
  [
    "holds a borrow of a RefCell the program borrows unchecked while the program's code runs",
    `#[test]
#[should_panic(expected = "already borrowed")]
fn held() {
    let counter = app::counter();
    let held = counter.n.borrow();
    app::bump(&counter);
    drop(held);
}
`,
    "borrows `app::Counter::n` unchecked",
  ],
])("an integration test that %s is refused", (_, rust, message) => {
  const { code, said } = rustJsTest(testCrate(`rust-js-test-program-${message.length}`, { "src/lib.rs": lib, "tests/held.rs": rust }));
  expect(code, said).not.toBe(0);
  expect(said).toContain(message);
});
