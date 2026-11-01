// Which cells a held borrow can be of (ADR 0863, amended, issue #445): a
// cell's flow, where it's made and where it goes, and a JS collection's
// mark that it keeps what it's given only in itself (ADR 1050). Each test
// is a program of its own, compiled and run.

import { beforeAll, expect, test } from "bun:test";
import { readFileSync, writeFileSync } from "node:fs";
import { join } from "node:path";
import { buildReact, compiler, fixture, run, target } from "./support";

beforeAll(buildReact, 600_000);

const withBindings = [
  "--",
  "--extern",
  `js=${join(target, "libjs.rmeta")}`,
  "-L",
  target,
];

// ADR 1050: the mark is a binding's that runs nothing, of a JS object.
test("a collection's mark is refused where it can't be said", () => {
  const dir = fixture("collection-mark-refused");
  writeFileSync(join(dir, "lib.rs"), `use std::marker::PhantomData;

pub struct Bag<T>(PhantomData<js::JsObject>, PhantomData<T>);

impl<T> Bag<T> {
    #[cfg_attr(rust_js, rust_js::link_name = "push")]
    #[cfg_attr(rust_js, rust_js::keeps_in_collection)]
    pub fn push(&self, value: T) {
        let _ = value;
        unreachable!()
    }
}

#[cfg_attr(rust_js, rust_js::keeps_in_collection)]
pub fn kept(n: u32) -> u32 {
    n
}

#[cfg_attr(rust_js, rust_js::link_name = "globalThis.twice")]
#[cfg_attr(rust_js, rust_js::runs_nothing)]
#[cfg_attr(rust_js, rust_js::keeps_in_collection)]
pub fn twice(n: u32) -> u32 {
    let _ = n;
    unreachable!()
}
`);
  const p = Bun.spawnSync([compiler, join(dir, "lib.rs"), "-o", join(dir, "lib.js"), ...withBindings], {
    stderr: "pipe",
  });
  expect(p.exitCode).not.toBe(0);
  const stderr = p.stderr.toString();
  expect(stderr).toContain("it says `#[rust_js::runs_nothing]` too");
  expect(stderr).toContain("this one's body is the program's");
  expect(stderr).toContain("this one has neither");
});

// The text of the function `name` the module `js` exports.
const fn = (js: string, name: string) => {
  const at = js.indexOf(`export function ${name}(`);
  expect(at).toBeGreaterThanOrEqual(0);
  const end = js.indexOf("\n}\n", at);
  return js.slice(at, end);
};

// ADR 0863, amended (issue #445, rd201): React DOM keeps each tag's cache
// of elements in a JS `Map` and holds one while it reads a prop's getter,
// which may call back and push to the same cache: that borrow is checked,
// and panics as Rust's does. Fizz's batch, a `RefCell` of the same type
// JS gives, is no cache any getter reaches: its loop is plain.
test("a held borrow counts the cells that can be its own, not every cell of its type", async () => {
  const dir = fixture("cell-flow-tag-cache");
  writeFileSync(join(dir, "lib.rs"), `pub mod hoist {
    use js::{Map, Unknown};
    use std::cell::{Cell, RefCell};
    use std::rc::Rc;

    type Cache = Map<str, Option<Rc<RefCell<Vec<u32>>>>>;

    thread_local! {
        static CACHES: Cell<Option<&'static Cache>> = const { Cell::new(None) };
    }

    #[cfg_attr(rust_js, rust_js::link_name = "globalThis.matches")]
    fn matches(props: &Unknown, node: u32) -> bool {
        let _ = (props, node);
        unreachable!()
    }

    fn cache() -> &'static Cache {
        match CACHES.get() {
            Some(cache) => cache,
            None => {
                let cache = js::map::new(None);
                CACHES.set(Some(cache));
                cache
            }
        }
    }

    pub fn seed(key: &str, node: u32) {
        let cache = cache();
        if let Some(nodes) = cache.get(key).flatten() {
            nodes.borrow_mut().push(node);
        } else {
            cache.set(key, Some(Rc::new(RefCell::new(vec![node]))));
        }
    }

    pub fn hydrate(key: &str, props: &Unknown) -> Option<u32> {
        let nodes = cache().get(key).flatten()?;
        let mut nodes = nodes.borrow_mut();
        for i in 0..nodes.len() {
            if matches(props, nodes[i]) {
                return Some(nodes.remove(i));
            }
        }
        None
    }
}

pub mod batch {
    use std::cell::RefCell;

    pub type Batch = RefCell<Vec<u32>>;

    unsafe extern "Rust" {
        #[link_name = "globalThis.$RB"]
        safe static RB: &'static Batch;
    }

    pub fn reveal(batch: &Batch) -> u32 {
        let mut total = 0;
        for i in (0..batch.borrow().len()).step_by(2) {
            total += batch.borrow()[i];
        }
        batch.borrow_mut().clear();
        total
    }

    pub fn reveal_pending() -> u32 {
        reveal(RB)
    }
}
`);
  run([compiler, join(dir, "lib.rs"), "-o", join(dir, "lib.js"), ...withBindings]);
  // The batch's borrows: the array itself.
  expect(fn(readFileSync(join(dir, "batch.js"), "utf8"), "reveal")).toBe(`export function reveal(batch) {
  let total = 0;
  for (let i = 0; i < batch.length; i += 2) {
    total = (total + batch[i]) >>> 0;
  }
  batch.length = 0;
  return total;`);
  // The tag cache's, held while a getter runs: checked.
  expect(fn(readFileSync(join(dir, "hoist.js"), "utf8"), "hydrate")).toContain("$borrowMut(nodes, true)");
  const lib = { ...(await import(join(dir, "batch.js"))), ...(await import(join(dir, "hoist.js"))) };
  const globals = globalThis as any;
  try {
    globals.$RB = [1, 2, 3, 4];
    expect(lib.reveal([5, 6, 7])).toBe(12);
    expect(lib.revealPending()).toBe(4);
    expect(globals.$RB).toEqual([]);
    lib.seed("link", 1);
    lib.seed("link", 2);
    globals.matches = (_props: unknown, node: number) => node === 2;
    expect(lib.hydrate("link", {})).toBe(2);
    // A getter that pushes to the cache it's held of: Rust panics.
    globals.matches = () => (lib.seed("link", 3), false);
    expect(() => lib.hydrate("link", {})).toThrow("already borrowed");
  } finally {
    delete globals.$RB;
    delete globals.matches;
  }
});

// ADR 0863, amended: a cell JS gives an exported function may be any cell
// JS reaches, the batch a binding gives among them. One held while JS runs
// is checked, and so is each borrow of a cell JS may reach of its type,
// which panics as Rust's does.
test("a cell JS gives may be any cell of its type JS reaches", async () => {
  const dir = fixture("cell-flow-given");
  writeFileSync(join(dir, "lib.rs"), `use std::cell::RefCell;

pub type Batch = RefCell<Vec<u32>>;

unsafe extern "Rust" {
    #[link_name = "globalThis.$RB"]
    safe static RB: &'static Batch;
}

#[cfg_attr(rust_js, rust_js::link_name = "globalThis.touch")]
fn touch() {
    unreachable!()
}

pub fn hold(batch: &Batch) {
    let mut held = batch.borrow_mut();
    held.push(0);
    touch();
    held.push(1);
}

pub fn reveal_pending() -> usize {
    RB.borrow().len()
}
`);
  run([compiler, join(dir, "lib.rs"), "-o", join(dir, "lib.js"), ...withBindings]);
  const js = readFileSync(join(dir, "lib.js"), "utf8");
  expect(fn(js, "revealPending")).toContain("$borrow(");
  const lib = await import(join(dir, "lib.js"));
  const globals = globalThis as any;
  try {
    globals.$RB = [7];
    globals.touch = () => {};
    lib.hold(globals.$RB);
    expect(lib.revealPending()).toBe(3);
    globals.touch = () => lib.revealPending();
    expect(() => lib.hold(globals.$RB)).toThrow("already mutably borrowed");
  } finally {
    delete globals.$RB;
    delete globals.touch;
  }
});

// ADR 0863, amended: a cell given to JS, by a binding that may keep it
// anywhere, may come back as any cell JS gives of its type.
test("a cell given to JS may be any cell JS gives back", async () => {
  const dir = fixture("cell-flow-kept");
  writeFileSync(join(dir, "lib.rs"), `use std::cell::RefCell;
use std::rc::Rc;

#[cfg_attr(rust_js, rust_js::link_name = "globalThis.keep")]
fn keep(cell: Rc<RefCell<Vec<u32>>>) {
    let _ = cell;
    unreachable!()
}

#[cfg_attr(rust_js, rust_js::link_name = "globalThis.kept")]
fn kept() -> Rc<RefCell<Vec<u32>>> {
    unreachable!()
}

#[cfg_attr(rust_js, rust_js::link_name = "globalThis.poke")]
fn poke() {
    unreachable!()
}

pub fn hold() -> usize {
    let mine = Rc::new(RefCell::new(vec![1]));
    keep(Rc::clone(&mine));
    let mut held = mine.borrow_mut();
    held.push(2);
    poke();
    held.len()
}

pub fn bump() -> usize {
    let cell = kept();
    cell.borrow().len()
}
`);
  run([compiler, join(dir, "lib.rs"), "-o", join(dir, "lib.js"), ...withBindings]);
  const js = readFileSync(join(dir, "lib.js"), "utf8");
  expect(fn(js, "bump")).toContain("$borrow(cell)");
  const lib = await import(join(dir, "lib.js"));
  const globals = globalThis as any;
  let kept: unknown;
  try {
    globals.keep = (cell: unknown) => (kept = cell);
    globals.kept = () => kept;
    globals.poke = () => {};
    expect(lib.hold()).toBe(2);
    globals.poke = () => lib.bump();
    expect(() => lib.hold()).toThrow("already mutably borrowed");
  } finally {
    delete globals.keep;
    delete globals.kept;
    delete globals.poke;
  }
});
