// A class's constructor (ADR 0761): its statements are its constructor
// function's, the instance it makes `this`, and what it gives, `Self` or an
// `Rc` of it, the instance `new` makes. Split from classes.test.ts by
// concept, as a test file is at most 1500 lines. Each test is a program of
// its own, compiled and run in Node.

import { beforeAll, expect, test } from "bun:test";
import { readFileSync, writeFileSync } from "node:fs";
import { join } from "node:path";
import { runSync } from "./child";
import { buildReact, compiler, fixture, root, run, target } from "./support";

beforeAll(buildReact, 600_000);

const flags = (dir: string) => ["--declarations", join(dir, "lib.rs"), "-o", join(dir, "lib.js"), "--", "--extern", `js=${join(target, "libjs.rmeta")}`, "-L", target];

/** `rust` compiled in a fixture of `name`, against the js crate: its JS, its declarations, and where they are. */
function compiled(name: string, rust: string): { js: string; ts: string; dir: string } {
  const dir = fixture(name);
  writeFileSync(join(dir, "lib.rs"), rust);
  run([compiler, ...flags(dir)]);
  return { js: readFileSync(join(dir, "lib.js"), "utf8"), ts: readFileSync(join(dir, "lib.d.ts"), "utf8"), dir };
}

/** What the compiler says of `rust`, which it refuses. */
function refused(name: string, rust: string): string {
  const dir = fixture(name);
  writeFileSync(join(dir, "lib.rs"), rust);
  const ran = runSync([compiler, ...flags(dir)], root, 60_000);
  expect(ran.code).not.toBe(0);
  return ran.stderr;
}

// ADR 0761, amended: React's `function FiberNode(tag, ..) { this.tag =
// tag; ..; if (enableProfilerTimer) { this.actualDuration = -0; } if
// (__DEV__) { this._debugOwner = null; Object.preventExtensions(this); }
// }`. A constructor's statements are the constructor function's: the
// variable it returns, made of a struct literal, is `this`, its fields set
// where it's made, a `None`'s left out as an object's is (ADR 0280), and
// each read of it `this`, a handle on its field's place the instance's,
// named. The struct's `#[rust_js::name]` is the class's, `FiberNode` of
// `Fiber`, declared where its constructor function is, though its struct
// is another module's, ReactInternalTypes'.
test("a constructor's statements set this, of a class named apart from its struct, in another module", async () => {
  const dir = fixture("class-constructor-statements");
  writeFileSync(join(dir, "lib.rs"), `pub mod internal_types {
    use std::cell::Cell;

    use js::Unknown;

    /// A fiber.
    #[cfg_attr(rust_js, rust_js::class, rust_js::name = "FiberNode")]
    pub struct Fiber {
        pub tag: Cell<u8>,
        pub memoized_state: Cell<Option<&'static Unknown>>,
        #[cfg_attr(rust_js, rust_js::nullable, rust_js::optional)]
        pub actual_duration: Cell<Option<f64>>,
        #[cfg_attr(rust_js, rust_js::name = "_debugOwner")]
        #[cfg_attr(rust_js, rust_js::nullable, rust_js::optional)]
        pub debug_owner: Cell<Option<&'static Unknown>>,
        pub reads: Cell<u32>,
    }

    unsafe impl js::Defined for Fiber {}
}

pub mod fiber {
    use std::cell::Cell;

    use crate::internal_types::Fiber;

    thread_local! {
        static ENABLE_PROFILER_TIMER: Cell<bool> = const { Cell::new(true) };
    }

    fn bump(count: &Cell<u32>) {
        count.set(count.get() + 1);
    }

    #[cfg_attr(rust_js, rust_js::class)]
    impl Fiber {
        pub fn new(tag: u8, dev: bool) -> Self {
            let fiber = Fiber {
                tag: Cell::new(tag),
                memoized_state: Cell::new(js::null()),
                actual_duration: Cell::new(None),
                debug_owner: Cell::new(None),
                reads: Cell::new(0),
            };
            if ENABLE_PROFILER_TIMER.get() {
                fiber.actual_duration.set(Some(-0.0));
            }
            if dev {
                fiber.debug_owner.set(js::null());
                bump(&fiber.reads);
                js::object::prevent_extensions(&fiber);
            }
            fiber
        }

        pub fn work_tag(&self) -> u8 {
            self.tag.get()
        }
    }

    pub fn create_fiber(tag: u8, dev: bool) -> Fiber {
        Fiber::new(tag, dev)
    }

    pub fn profile(on: bool) {
        ENABLE_PROFILER_TIMER.set(on);
    }
}
`);
  run([compiler, ...flags(dir)]);
  const js = readFileSync(join(dir, "fiber.js"), "utf8");
  const ts = readFileSync(join(dir, "fiber.d.ts"), "utf8");
  expect(js).toContain(`/** A fiber. */
export function FiberNode(tag, dev) {
  const fiberNode = this;
  this.tag = tag;
  this.memoizedState = null;
  this.reads = 0;
  if (enableProfilerTimer) {
    this.actualDuration = -0;
  }
  if (dev) {
    this._debugOwner = null;
`);
  expect(js).toContain("    Object.preventExtensions(this);\n  }\n}\n");
  expect(js).toContain("  return new FiberNode(tag, dev);\n");
  expect(ts).toContain("export declare class FiberNode {\n    constructor(tag: number, dev: boolean);\n");
  expect(ts).toContain("export function createFiber(tag: number, dev: boolean): FiberNode;\n");
  const { FiberNode, createFiber, profile } = await import(join(dir, "fiber.js"));
  const fiber = createFiber(3, true);
  expect(fiber).toBeInstanceOf(FiberNode);
  expect([fiber.workTag(), fiber.reads, Object.is(fiber.actualDuration, -0), fiber._debugOwner, Object.isExtensible(fiber)])
    .toEqual([3, 1, true, null, false]);
  profile(false);
  const plain = createFiber(5, false);
  expect(["actualDuration" in plain, "_debugOwner" in plain, Object.isExtensible(plain)]).toEqual([false, false, true]);
});

// React's Flight server makes its request with `new RequestInstance(..)`,
// whose constructor hands `this` to its root task, which keeps it:
// `createTask(this, ..)`. A Rust engineer's constructor gives the `Rc` it
// shares, `fn new(..) -> Rc<Self>`; an `Rc` the crate counts nothing of is
// the value it points at, so the constructor's `Rc::new(Request { .. })`
// is `this`, each read of it `this`, and `Request::new(..)` is `new
// RequestInstance(..)` (rd554, ADR 0761 amended).
test("a class's constructor that gives an Rc of it makes this, which it hands on", async () => {
  const { js, ts, dir } = compiled("class-constructor-rc", `use std::cell::{Cell, RefCell};
use std::rc::Rc;

pub struct Task {
    pub request: Rc<Request>,
    pub id: u32,
}

#[cfg_attr(rust_js, rust_js::class, rust_js::name = "RequestInstance")]
pub struct Request {
    pub kind: u8,
    pub next_id: Cell<u32>,
    pub pinged_tasks: RefCell<Vec<Rc<Task>>>,
}

unsafe impl js::Defined for Request {}

fn create_task(request: &Rc<Request>) -> Rc<Task> {
    let id = request.next_id.get();
    request.next_id.set(id + 1);
    Rc::new(Task {
        request: Rc::clone(request),
        id,
    })
}

#[cfg_attr(rust_js, rust_js::class)]
impl Request {
    fn new(
        kind: u8,
        tasks: u32, // the tasks it starts with
    ) -> Rc<Self> {
        let request = Rc::new(Request {
            kind,
            next_id: Cell::new(0),
            pinged_tasks: RefCell::new(vec![]),
        });
        if kind == 0 {
            return request;
        }
        for _ in 0..tasks {
            let task = create_task(&request);
            request.pinged_tasks.borrow_mut().push(task);
        }
        request
    }

    pub fn pinged(&self) -> u32 {
        self.pinged_tasks.borrow().len() as u32
    }
}

#[cfg_attr(rust_js, rust_js::class)]
pub struct Hints {
    pub count: Cell<u32>,
}

#[cfg_attr(rust_js, rust_js::class)]
impl Hints {
    pub fn new() -> Rc<Self> {
        Rc::new(Hints { count: Cell::new(0) })
    }
}

pub fn create_request(kind: u8, tasks: u32) -> Rc<Request> {
    // The request starts its tasks.
    Request::new(kind, tasks)
}

pub fn held_by_its_tasks(request: &Rc<Request>) -> bool {
    (request.pinged_tasks.borrow().iter()).all(|task| std::ptr::eq(&*task.request, &**request))
}

pub fn hinted() -> u32 {
    let hints = Hints::new();
    hints.count.set(hints.count.get() + 2);
    hints.count.get()
}
`);
  expect(js).toContain(`function RequestInstance(
  kind,
  tasks, // the tasks it starts with
) {
  this.kind = kind;
  this.nextId = 0;
  this.pingedTasks = [];
  if (kind === 0) {
    return;
  }
`);
  expect(js).toContain("const task = createTask(this);\n");
  expect(js).toContain("this.pingedTasks.push(task);\n");
  expect(js).toContain("  // The request starts its tasks.\n  return new RequestInstance(kind, tasks);\n");
  expect(js).toContain("export function Hints() {\n  this.count = 0;\n}\n");
  expect(js).toContain("const hints = new Hints();\n");
  expect(ts).toContain("export function createRequest(kind: number, tasks: number): RequestInstance;\n");
  const { createRequest, heldByItsTasks, hinted, Hints } = await import(join(dir, "lib.js"));
  const request = createRequest(1, 3);
  expect([request.pinged(), request.nextId, heldByItsTasks(request), request.pingedTasks.map((task: { id: number }) => task.id)])
    .toEqual([3, 3, true, [0, 1, 2]]);
  expect(request.pingedTasks[0].request).toBe(request);
  expect(request.constructor.name).toBe("RequestInstance");
  expect(createRequest(0, 3).pinged()).toBe(0);
  expect(hinted()).toBe(2);
  expect(new Hints()).toBeInstanceOf(Hints);
});

// An `Rc` the crate counts of a class, whose counts it reads, is the
// instance itself, its counts hidden beside it, `$countedInstance(this)` (#1228,
// the user's decision): `new` gives it, JS calls its methods on it, a
// method may take `self: &Rc<Self>`, and JS sees none of its counts, not in
// its keys, its JSON or a spread of it.
test("a class's constructor that gives an Rc of it the crate counts gives the instance", async () => {
  const { js, ts, dir } = compiled("class-constructor-rc-counted", `use std::cell::Cell;
use std::rc::Rc;

#[cfg_attr(rust_js, rust_js::class)]
pub struct Counter {
    pub count: Cell<u32>,
}

#[cfg_attr(rust_js, rust_js::class)]
impl Counter {
    pub fn new() -> Rc<Self> {
        Rc::new(Counter { count: Cell::new(0) })
    }

    pub fn owned(self: &Rc<Self>) -> u32 {
        self.count.set(self.count.get() + 1);
        Rc::strong_count(self) as u32
    }
}

pub fn owners() -> usize {
    let counter = Counter::new();
    let again = Rc::clone(&counter);
    Rc::strong_count(&again)
}

pub fn shared() -> Rc<Counter> {
    let counter = Counter::new();
    counter.owned();
    counter
}
`);
  expect(js).toContain("export function Counter() {\n  this.count = 0;\n  $countedInstance(this);\n}\n");
  expect(js).toContain("  return $countedBox(this).strong;\n");
  expect(ts).toContain("export function shared(): Counter;\n");
  const { Counter, owners, shared } = await import(join(dir, "lib.js"));
  const counter = new Counter();
  expect([counter instanceof Counter, Object.keys(counter), JSON.stringify(counter), Object.keys({ ...counter })])
    .toEqual([true, ["count"], '{"count":0}', ["count"]]);
  expect([counter.owned(), counter.count, owners()]).toEqual([1, 1, 2]);
  const made = shared();
  expect([made instanceof Counter, made.count, made.owned()]).toEqual([true, 1, 1]);
});
