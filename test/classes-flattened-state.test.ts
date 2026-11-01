// A class's state flattened into it, a `RefCell` of a discriminated union
// (rd380, ADR 1044), as React's Flight client's chunk has `status`, `value`
// and `reason` on itself: its constructor takes the state's keys, each
// `new` gives them, its borrows are its object's own keys, and a whole
// write sets each. The corpus's `flattened_state_cell` runs one against
// native Rust; these hold the Flight client's form and what's refused.

import { beforeAll, expect, test } from "bun:test";
import { readFileSync, writeFileSync } from "node:fs";
import { join } from "node:path";
import { runSync } from "./child";
import { buildReact, compiler, fixture, root, target } from "./support";

beforeAll(buildReact, 600_000);

const flags = (dir: string) => [join(dir, "lib.rs"), "-o", join(dir, "lib.js"), "--", "--extern", `js=${join(target, "libjs.rmeta")}`, "-L", target];

/** `rust` compiled against the js crate: its JS, and its module, loaded. */
async function compiled(name: string, rust: string): Promise<{ js: string; lib: any }> {
  const dir = fixture(name);
  writeFileSync(join(dir, "lib.rs"), rust);
  const ran = runSync([compiler, ...flags(dir)], root, 600_000);
  expect(ran.code, ran.stderr).toBe(0);
  return { js: readFileSync(join(dir, "lib.js"), "utf8"), lib: await import(join(dir, "lib.js")) };
}

/** What the compiler says of `rust`, which it refuses. */
function refused(name: string, rust: string): string {
  const dir = fixture(name);
  writeFileSync(join(dir, "lib.rs"), rust);
  const ran = runSync([compiler, ...flags(dir)], root, 600_000);
  expect(ran.code).not.toBe(0);
  return ran.stderr;
}

const state = `use std::cell::RefCell;

use js::Unknown;

#[cfg_attr(rust_js, rust_js::tag = "status")]
pub enum ChunkState {
    #[cfg_attr(rust_js, rust_js::name = "pending")]
    Pending {
        #[cfg_attr(rust_js, rust_js::nullable)]
        value: Option<Vec<&'static dyn Fn(Option<&'static Unknown>)>>,
        #[cfg_attr(rust_js, rust_js::nullable)]
        reason: Option<Vec<&'static dyn Fn(Option<&'static Unknown>)>>,
    },
    #[cfg_attr(rust_js, rust_js::name = "fulfilled")]
    Initialized {
        value: Option<&'static Unknown>,
        #[cfg_attr(rust_js, rust_js::nullable)]
        reason: Option<&'static Unknown>,
    },
    #[cfg_attr(rust_js, rust_js::name = "rejected")]
    Errored { reason: Option<&'static Unknown> },
}

#[cfg_attr(rust_js, rust_js::class)]
pub struct ReactPromise {
    #[cfg_attr(rust_js, rust_js::flatten)]
    pub state: RefCell<ChunkState>,
}

unsafe impl js::Defined for ReactPromise {}

#[cfg_attr(rust_js, rust_js::class)]
impl ReactPromise {
    fn new(state: ChunkState) -> Self {
        ReactPromise {
            state: RefCell::new(state),
        }
    }
}
`;

// ReactFlightClient's createPendingChunk, wakeChunk's resolve and
// readChunk: upstream's `new ReactPromise(PENDING, null, null)`,
// `chunk.status = INITIALIZED; chunk.value = value;` and `throw
// chunk.reason`, the chunk itself the state's object.
test("a flattened state is its class's own keys, made, read and replaced in place", async () => {
  const { js, lib } = await compiled(
    "flattened-state-flight",
    `${state}
pub fn create_pending_chunk() -> std::rc::Rc<ReactPromise> {
    std::rc::Rc::new(ReactPromise::new(ChunkState::Pending {
        value: None,
        reason: None,
    }))
}

pub fn create_errored_chunk(reason: Option<&'static Unknown>) -> std::rc::Rc<ReactPromise> {
    std::rc::Rc::new(ReactPromise::new(ChunkState::Errored { reason }))
}

pub fn listen(chunk: &ReactPromise, listener: &'static dyn Fn(Option<&'static Unknown>)) {
    if let ChunkState::Pending { value, .. } = &mut *chunk.state.borrow_mut() {
        value.get_or_insert_with(Vec::new).push(listener);
    }
}

pub fn resolve(chunk: &ReactPromise, value: Option<&'static Unknown>) {
    let listeners = match &mut *chunk.state.borrow_mut() {
        ChunkState::Pending { value, .. } => value.take(),
        _ => None,
    };
    *chunk.state.borrow_mut() = ChunkState::Initialized { value, reason: None };
    for listener in listeners.unwrap_or_default() {
        listener(value);
    }
}

pub fn read(chunk: &ReactPromise) -> Option<&'static Unknown> {
    match &*chunk.state.borrow() {
        ChunkState::Initialized { value, .. } => *value,
        ChunkState::Pending { .. } => js::throw(Some(js::unknown(chunk))),
        ChunkState::Errored { reason } => js::throw(*reason),
    }
}
`,
  );
  expect(js).toContain(`function ReactPromise(status, value, reason) {
  this.status = status;
  this.value = value;
  this.reason = reason;
}`);
  expect(js).toContain('new ReactPromise("pending", null, null)');
  expect(js).toContain('  chunk.status = "fulfilled";\n  chunk.value = value;\n  chunk.reason = null;\n');
  expect(js).toContain("throw chunk.reason;");
  expect(js).toContain('new ReactPromise("rejected", undefined, reason)');
  const errored = lib.createErroredChunk("bad");
  expect([errored.status, errored.value, errored.reason]).toEqual(["rejected", undefined, "bad"]);
  expect(js).not.toContain("$borrow");
  const chunk = lib.createPendingChunk();
  expect([chunk.status, chunk.value, chunk.reason, chunk instanceof Object]).toEqual(["pending", null, null, true]);
  expect(() => lib.read(chunk)).toThrow();
  const seen: unknown[] = [];
  lib.listen(chunk, (v: unknown) => seen.push(v));
  lib.resolve(chunk, 7);
  expect([chunk.status, chunk.value, chunk.reason, seen, lib.read(chunk)]).toEqual(["fulfilled", 7, null, [7], 7]);
  chunk.status = "rejected";
  chunk.reason = "no";
  let thrown;
  try {
    lib.read(chunk);
  } catch (e) {
    thrown = e;
  }
  expect(thrown).toBe("no");
});

// What would give or take the state whole, which is no object of its own,
// is refused, and so is one lent or borrowed where another borrow may meet
// it, which its object can't count.
test("a flattened state given or taken whole, lent or held is refused", () => {
  expect(
    refused(
      "flattened-state-replace",
      `${state}
pub fn replaced(chunk: &ReactPromise) -> bool {
    matches!(chunk.state.replace(ChunkState::Errored { reason: None }), ChunkState::Pending { .. })
}
`,
    ),
  ).toContain("a flattened state's value given or taken whole");
  expect(
    refused(
      "flattened-state-lent",
      `${state}
pub fn lent(chunk: &ReactPromise) -> &RefCell<ChunkState> {
    &chunk.state
}
`,
    ),
  ).toContain("a flattened state lent, or borrowed where another borrow of it may be");
  expect(
    refused(
      "flattened-state-held",
      `${state}
pub fn held(chunk: &ReactPromise, f: &dyn Fn()) -> bool {
    let state = chunk.state.borrow();
    f();
    matches!(*state, ChunkState::Pending { .. })
}
`,
    ),
  ).toContain("a flattened state lent, or borrowed where another borrow of it may be");
  expect(
    refused(
      "flattened-state-plain-cell",
      `use std::cell::RefCell;

pub struct Inner {
    pub n: u32,
}

pub struct Outer {
    #[cfg_attr(rust_js, rust_js::flatten)]
    pub inner: RefCell<Inner>,
}

pub fn n(outer: &Outer) -> u32 {
    outer.inner.borrow().n
}
`,
    ),
  ).toContain("a flattened field is a struct of named fields, a JS object or a `RefCell` of a `#[rust_js::tag]` enum");
});

// rd430: an or-pattern of a field each variant has at one key,
// `Pending { value } | Blocked { value }`, `&mut` through the state's
// borrow, is one test of either, its arm once, the field `chunk.value`
// read and written there, as upstream's `case PENDING: case BLOCKED:`;
// not a getter and setter box each alternative sets, nor a `match`'s
// labeled block.
test("an or-pattern of a field each variant has is its key, one arm", async () => {
  const { js, lib } = await compiled("flattened-state-or-pattern", `use std::cell::RefCell;
use std::rc::Rc;

#[cfg_attr(rust_js, rust_js::tag = "status")]
pub enum ChunkState {
    #[cfg_attr(rust_js, rust_js::name = "pending")]
    Pending {
        #[cfg_attr(rust_js, rust_js::nullable)]
        value: Option<Vec<u32>>,
    },
    #[cfg_attr(rust_js, rust_js::name = "blocked")]
    Blocked {
        #[cfg_attr(rust_js, rust_js::nullable)]
        value: Option<Vec<u32>>,
    },
    #[cfg_attr(rust_js, rust_js::name = "fulfilled")]
    Initialized { value: String },
}

#[cfg_attr(rust_js, rust_js::class)]
pub struct ReactPromise {
    #[cfg_attr(rust_js, rust_js::flatten)]
    pub state: RefCell<ChunkState>,
}

#[cfg_attr(rust_js, rust_js::class)]
impl ReactPromise {
    fn new(state: ChunkState) -> Self {
        ReactPromise { state: RefCell::new(state) }
    }
}

// Upstream: \`case PENDING: case BLOCKED: if (chunk.value === null) chunk.value = [id]; else chunk.value.push(id);\`
pub fn listen(chunk: &Rc<ReactPromise>, id: u32) {
    if let ChunkState::Pending { value } | ChunkState::Blocked { value } = &mut *chunk.state.borrow_mut() {
        match value {
            None => *value = Some(vec![id]),
            Some(listeners) => listeners.push(id),
        }
    }
}

// Upstream: \`const listeners = chunk.value; chunk.status = 'fulfilled'; chunk.value = text;\`
pub fn take(chunk: &Rc<ReactPromise>, text: String) -> Option<Vec<u32>> {
    let listeners = match &mut *chunk.state.borrow_mut() {
        ChunkState::Pending { value } | ChunkState::Blocked { value } => value.take(),
        _ => None,
    };
    *chunk.state.borrow_mut() = ChunkState::Initialized { value: text };
    listeners
}

pub fn make() -> Rc<ReactPromise> {
    Rc::new(ReactPromise::new(ChunkState::Pending { value: None }))
}
`);
  expect(js).toContain('  if (chunk.status === "pending" || chunk.status === "blocked") {\n');
  expect(js).not.toContain("get value()");
  expect(js).not.toMatch(/\bbb\d+:/);
  const chunk = lib.make();
  lib.listen(chunk, 1);
  lib.listen(chunk, 2);
  expect(chunk.value).toEqual([1, 2]);
  expect(lib.take(chunk, "done")).toEqual([1, 2]);
  expect([chunk.status, chunk.value]).toEqual(["fulfilled", "done"]);
  lib.listen(chunk, 3);
  expect([lib.take(chunk, "again"), chunk.value]).toEqual([undefined, "again"]);
});
