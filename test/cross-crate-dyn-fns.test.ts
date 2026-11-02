// What a crate's and its libraries' `dyn Fn`s drop: a library's contract
// says whether it makes one of what drops, and a crate that would hand one
// that drops to a library whose `dyn Fn`s drop nothing is refused (ADRs
// 0521, 0626). Each test compiles a library and a crate that uses it, and
// runs what it wrote.

import { beforeAll, expect, test } from "bun:test";
import { readFileSync, writeFileSync } from "node:fs";
import { join } from "node:path";
import { buildReact, compiler, fixture, run, target } from "./support";

beforeAll(buildReact, 600_000);

const js = ["--extern", `js=${join(target, "libjs.rmeta")}`, "-L", target];

/** `dep`, a library, and `app`, a crate that uses it, compiled in `dir`:
 * each one's JS. */
function compiled(dir: string, dep: string, app: string): { dep: string; app: string } {
  writeFileSync(join(dir, "dep.rs"), dep);
  run([compiler, join(dir, "dep.rs"), "-o", join(dir, "dep", "lib.js"), "--library", "--manifest", join(dir, "dep", "lib.manifest.json"),
    "--", "--crate-name", "dep", `--emit=metadata=${join(dir, "libdep.rmeta")}`, ...js]);
  writeFileSync(join(dir, "app.rs"), app);
  run([compiler, join(dir, "app.rs"), "-o", join(dir, "app", "lib.js"), "--dependency", join(dir, "dep", "lib.manifest.json"),
    "--", "--crate-name", "app", "--extern", `dep=${join(dir, "libdep.rmeta")}`, "-L", dir, ...js]);
  return { dep: readFileSync(join(dir, "dep", "lib.js"), "utf8"), app: readFileSync(join(dir, "app", "lib.js"), "utf8") };
}

// React's react_isomorphic and shared hand the reconciler `dyn Fn()`s, made
// of none that drops, and the reconciler's chainThenableValue<T> boxes a
// closure that holds only its listeners. Neither makes the crate's `dyn Fn`s
// drop: a library's contract says whether it makes a `dyn Fn` of what
// drops, and a closure of a generic function's is judged by what it holds
// (ADRs 0521, 0626, amended).
test("a library's dyn Fn drops in its consumer only where a library makes one of what drops", async () => {
  const dir = fixture("cross-crate-dyn-fn-plain");
  const { app } = compiled(dir, `pub fn call(f: &dyn Fn() -> u32) -> u32 {
    f()
}

pub fn counter() -> Box<dyn Fn() -> u32> {
    Box::new(|| 1)
}

// React's useReducer, a \`dyn Fn\` of its type parameters given on.
pub fn use_reducer<S, A>(reducer: &'static dyn Fn(S, A) -> S, state: S, action: A) -> S {
    reduce(reducer, state, action)
}

fn reduce<S, A>(reducer: &dyn Fn(S, A) -> S, state: S, action: A) -> S {
    reducer(state, action)
}

// A closure that holds a type parameter's value, made no \`dyn Fn\`.
pub fn repeat<T: Clone>(value: T, n: usize) -> Vec<T> {
    (0..n).map(move |_| value.clone()).collect()
}
`, `use std::cell::RefCell;
use std::rc::Rc;

type Listener = Rc<dyn Fn()>;

pub fn controller() -> (Box<dyn Fn(Listener)>, Box<dyn Fn() -> u32>) {
    let listeners: Rc<RefCell<Vec<Listener>>> = Rc::new(RefCell::new(Vec::new()));
    let add: Box<dyn Fn(Listener)> = {
        let listeners = Rc::clone(&listeners);
        Box::new(move |listener| listeners.borrow_mut().push(listener))
    };
    let abort: Box<dyn Fn() -> u32> = Box::new(move || {
        let count = listeners.borrow().len();
        for index in 0..count {
            let listener = Rc::clone(&listeners.borrow()[index]);
            listener();
        }
        dep::call(&|| count as u32) + dep::counter()() + dep::use_reducer(&|s: u32, a: u32| s + a, 1, 2)
    });
    (add, abort)
}

pub fn chained<T: Copy + 'static>(result: T) -> (T, Box<dyn Fn(Listener)>) {
    let listeners: Rc<RefCell<Vec<Listener>>> = Rc::new(RefCell::new(Vec::new()));
    (result, Box::new(move |listener| listeners.borrow_mut().push(listener)))
}

pub fn chained_count() -> u32 {
    let (n, add) = chained(2);
    add(Rc::new(|| {}));
    n
}
`);
  expect(JSON.parse(readFileSync(join(dir, "dep", "lib.manifest.json"), "utf8")).library.makes_dropping_fn_dyns).toBe(false);
  expect(app).not.toContain("strong");
  expect(app).not.toContain("$dropping");
  expect(app).not.toContain("$dispose");
  const { controller, chainedCount } = await import(join(dir, "app", "lib.js"));
  const [add, abort] = controller();
  const heard: string[] = [];
  add(() => heard.push("one"));
  add(() => heard.push("two"));
  expect([abort(), heard, chainedCount()]).toEqual([6, ["one", "two"], 2]);
});

// And where a library makes a `dyn Fn` of a closure that holds a value with
// a destructor, its contract says so, and the crate's `dyn Fn`s carry their
// drops: the one it hands the crate is dropped where Rust drops it.
test("a library's dyn Fn of what drops is dropped where its consumer drops it", async () => {
  const dir = fixture("cross-crate-dyn-fn-drops");
  const { app } = compiled(dir, `pub struct Loud(pub u32);

impl Drop for Loud {
    fn drop(&mut self) {
        println!("dropped {}", self.0);
    }
}

pub fn loud(n: u32) -> Box<dyn Fn() -> u32> {
    let loud = Loud(n);
    Box::new(move || {
        let loud = &loud;
        loud.0 + 1
    })
}
`, `pub fn run(n: u32) -> u32 {
    let f = dep::loud(n);
    let value = f();
    println!("called {value}");
    value
}
`);
  expect(JSON.parse(readFileSync(join(dir, "dep", "lib.manifest.json"), "utf8")).library.makes_dropping_fn_dyns).toBe(true);
  const logged: unknown[] = [];
  const log = console.log;
  console.log = (...args: unknown[]) => logged.push(args.join(" "));
  try {
    const { run } = await import(join(dir, "app", "lib.js"));
    expect(run(4)).toBe(5);
  } finally {
    console.log = log;
  }
  expect(logged).toEqual(["called 5", "dropped 4"]);
});

// A library's `dyn Fn`s drop where it makes one of what drops, as its
// contract says: one whose do drops the consumer's it's given where Rust
// drops it. One whose `dyn Fn`s drop nothing, given one by a consumer whose
// may hold what drops, would drop nothing of it: the consumer is refused,
// naming it (ADR 0626, amended).
test("a library drops a consumer's dyn Fn where its own drop, and refuses one it would drop nothing of", async () => {
  const loud = `pub struct Loud(pub u32);

impl Drop for Loud {
    fn drop(&mut self) {
        println!("dropped {}", self.0);
    }
}

`;
  const run = `${loud}pub fn run(n: u32) -> u32 {
    let loud = Loud(n);
    dep::call_boxed(Box::new(move || {
        let loud = &loud;
        loud.0 + 1
    }))
}
`;
  const callBoxed = `pub fn call_boxed(f: Box<dyn Fn() -> u32>) -> u32 {
    let value = f();
    println!("called {value}");
    value
}
`;
  const dir = fixture("cross-crate-dyn-fn-given");
  compiled(dir, `${callBoxed}
pub struct Tick(pub u32);

impl Drop for Tick {
    fn drop(&mut self) {
        println!("tick {}", self.0);
    }
}

pub fn ticker(n: u32) -> Box<dyn Fn() -> u32> {
    let tick = Tick(n);
    Box::new(move || {
        let tick = &tick;
        tick.0
    })
}
`, run);
  expect(JSON.parse(readFileSync(join(dir, "dep", "lib.manifest.json"), "utf8")).library.makes_dropping_fn_dyns).toBe(true);
  const logged: unknown[] = [];
  const log = console.log;
  console.log = (...args: unknown[]) => logged.push(args.join(" "));
  try {
    const app = await import(join(dir, "app", "lib.js"));
    expect(app.run(7)).toBe(8);
  } finally {
    console.log = log;
  }
  expect(logged).toEqual(["called 8", "dropped 7"]);
  const refused = fixture("cross-crate-dyn-fn-refused");
  expect(() => compiled(refused, callBoxed, run)).toThrow("this crate's `dyn Fn`s may hold a value with a destructor, which their drop runs, but `dep`, a rust-js library whose `dyn Fn`s drop nothing, holds one of them");
  expect(JSON.parse(readFileSync(join(refused, "dep", "lib.manifest.json"), "utf8")).library.makes_dropping_fn_dyns).toBe(false);
});

// rd231: React's hooks share a bound function and the Rust closures an
// `Rc<dyn Fn>` holds alike, `Rc::from(function::bind(..))`, an `Rc<&dyn
// Fn>` unsized to an `Rc<dyn Fn>`, and compare them, `Rc::ptr_eq`: one made
// of a `&dyn Fn`, a JS function already, is that function, which its own
// making answers for, as one made of a `dyn Fn` is (ADR 0959), so its
// counts aren't asked, its closures drop nothing, and a library whose `dyn
// Fn`s drop nothing may hold them (ADR 0626).
test("an Rc of a bound function, compared, makes no dyn Fn drop", async () => {
  const dir = fixture("cross-crate-dyn-fn-bound");
  const { app } = compiled(dir, `use std::rc::Rc;

pub fn call_shared(f: Rc<dyn Fn(u32) -> u32>) -> u32 {
    f(1)
}
`, `use std::rc::Rc;

fn add(by: u32, n: u32) -> u32 {
    by + n
}

fn shared(by: u32) -> Rc<dyn Fn(u32) -> u32> {
    Rc::from(js::function::bind(add, None, (by,)))
}

pub fn run(n: u32) -> u32 {
    let f = shared(n);
    let g = Rc::clone(&f);
    let same = Rc::ptr_eq(&f, &g);
    dep::call_shared(Rc::new(move |x| f(x) + u32::from(same)))
}
`);
  expect(app).toContain("add.bind(undefined, by)");
  const lib = await import(join(dir, "app", "lib.js"));
  expect(lib.run(4)).toBe(6);
  // One reference made two \`Rc\`s is two to Rust, whose \`ptr_eq\` says so,
  // as native Rust's does: counted.
  const twiceDir = fixture("cross-crate-dyn-fn-bound-twice");
  compiled(twiceDir, "pub fn one() -> u32 {\n    1\n}\n", `use std::rc::Rc;

fn add(by: u32, n: u32) -> u32 {
    by + n
}

pub fn twice(n: u32) -> (bool, u32) {
    let bound = js::function::bind(add, None, (n,));
    let x: Rc<dyn Fn(u32) -> u32> = Rc::from(bound);
    let y: Rc<dyn Fn(u32) -> u32> = Rc::from(bound);
    (Rc::ptr_eq(&x, &y), x(1) + y(dep::one()))
}
`);
  const twice = await import(join(twiceDir, "app", "lib.js"));
  expect(twice.twice(4)).toEqual([false, 10]);
});

// rd597: React's decodeBoundActionMetaData<M>, generic over the bundler's
// manifest, which its caller gives one whose drop may run, makes an
// `Rc::new(|_| {})` of a closure that captures nothing. What an `Rc` of a
// closure owns is what it captures, not its function's type arguments, which
// its type names: it's counted for neither, nor the `Rc<dyn Fn>` it's made,
// so the closures that hold one drop nothing, and react_isomorphic, whose
// `dyn Fn`s drop nothing, may hold them. One that captures a value of a type
// parameter given one that drops, or a `Box` of one, may drop, and is
// refused there (ADR 0626, amended).
test("a generic function's closure that holds none of its type parameters makes no dyn Fn drop", async () => {
  const dep = `use std::rc::Rc;

pub fn call_shared(f: Rc<dyn Fn(u32) -> u32>) -> u32 {
    f(3)
}
`;
  const manifest = `pub struct Manifest(pub u32);

impl Drop for Manifest {
    fn drop(&mut self) {
        println!("dropped {}", self.0);
    }
}
`;
  const dir = fixture("cross-crate-dyn-fn-generic");
  const { app } = compiled(dir, dep, `use std::rc::Rc;

${manifest}
fn decode<M: ?Sized>(_manifest: &M) -> Rc<dyn Fn(u32) -> u32> {
    Rc::new(|x| x + 1)
}

pub fn run() -> u32 {
    let manifest = Manifest(1);
    let decoded = decode(&manifest);
    dep::call_shared(Rc::new(move |x| decoded(x) * 2))
}
`);
  expect(app).not.toContain("strong");
  const lib = await import(join(dir, "app", "lib.js"));
  const logged: unknown[] = [];
  const log = console.log;
  console.log = (...args: unknown[]) => logged.push(args.join(" "));
  try {
    expect(lib.run()).toBe(8);
  } finally {
    console.log = log;
  }
  expect(logged).toEqual(["dropped 1"]);
  const held = (param: string, value: string) => `use std::rc::Rc;

${manifest}
fn hold<T: 'static>(value: ${param}) -> Rc<dyn Fn(u32) -> u32> {
    Rc::new(move |x| {
        let _held = &value;
        x + 1
    })
}

pub fn run() -> u32 {
    let decoded = hold(${value});
    dep::call_shared(Rc::new(move |x| decoded(x) * 2))
}
`;
  for (const [name, param, value] of [["value", "T", "Manifest(1)"], ["box", "Box<T>", "Box::new(Manifest(1))"]]) {
    expect(() => compiled(fixture(`cross-crate-dyn-fn-generic-${name}`), dep, held(param, value))).toThrow(
      "this crate's `dyn Fn`s may hold a value with a destructor, which their drop runs, but `dep`, a rust-js library whose `dyn Fn`s drop nothing, holds one of them",
    );
  }
  // An `Rc` of one that captures a `T` is counted: the last clone drops
  // what it captured, as native Rust's does.
  const counted = fixture("cross-crate-dyn-fn-generic-counted");
  compiled(counted, dep, `use std::rc::Rc;

${manifest}
fn hold<T: 'static>(value: T) -> u32 {
    let decoded = Rc::new(move |x: u32| {
        let _held = &value;
        x + 1
    });
    let again = Rc::clone(&decoded);
    drop(decoded);
    println!("one");
    let n = again(1);
    drop(again);
    println!("two");
    n
}

pub fn run() -> u32 {
    hold(Manifest(1))
}
`);
  const heard: unknown[] = [];
  console.log = (...args: unknown[]) => heard.push(args.join(" "));
  try {
    expect((await import(join(counted, "app", "lib.js"))).run()).toBe(2);
  } finally {
    console.log = log;
  }
  expect(heard).toEqual(["one", "dropped 1", "two"]);
});
