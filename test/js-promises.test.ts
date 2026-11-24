// A promise's handlers and executor as JS's: a catch or a then handler
// gives a value or a promise, an executor may set the variables around
// it, and callbacks made in it read the promise's const. Split from
// js-builtins.test.ts by concept (#1334).

import { beforeAll, expect, test } from "bun:test";
import { readFileSync, writeFileSync } from "node:fs";
import { join } from "node:path";
import { runSync } from "./child";
import { node } from "./programs";
import { buildReact, compiler, fixture, root, run, target } from "./support";
import { withoutBoundary } from "./without-boundary";

beforeAll(buildReact, 600_000);

// ADR 0698: a promise's `catch` handler gives what TypeScript's
// `onrejected` may, `TResult | PromiseLike<TResult>`: a value, a promise of
// one, or, of an `Option`'s, nothing. A `catch` whose promise nothing
// reads is a statement, which rustc warns nothing of.
test("a promise's catch handler gives a value or a promise", async () => {
  const dir = fixture("js-promise-catch");
  writeFileSync(join(dir, "lib.rs"), `use js::{Promise, Unknown, error, promise};
pub fn silenced(maybe_promise: &'static Unknown) {
    unsafe { js::cast::<Promise<Option<&'static Unknown>>>(Some(maybe_promise)) }.catch(|_| {});
}
pub async fn valued() -> u32 {
    promise::reject::<u32>(Some(error::new(Some("no"), None))).catch(|_| 7).await
}
pub async fn promised() -> String {
    promise::reject::<String>(Some(error::new(Some("no"), None))).catch(|e| promise::resolve(e.message())).await
}
pub async fn nothing() -> Option<u32> {
    promise::reject::<Option<u32>>(Some(error::new(Some("no"), None))).catch(|_| {}).await
}
`);
  const built = runSync([compiler, join(dir, "lib.rs"), "-o", join(dir, "lib.js"), "--", "--extern", `js=${join(target, "libjs.rmeta")}`, "-L", target], root, 600_000);
  expect(built.code, built.stderr).toBe(0);
  expect(built.stderr).not.toContain("warning");
  const js = withoutBoundary(readFileSync(join(dir, "lib.js"), "utf8"));
  expect(js).toContain("maybePromise.catch(() => {});");
  expect(js).toContain('return await Promise.reject(new Error("no")).catch(() => 7);');
  expect(js).toContain("catch(() => {})");
  writeFileSync(join(dir, "run.mjs"), `import * as lib from "./lib.js";
let unhandled = 0;
process.on("unhandledRejection", () => unhandled++);
lib.silenced(Promise.reject(new Error("async component")));
console.log(JSON.stringify([await lib.valued(), await lib.promised(), (await lib.nothing()) ?? null]));
await new Promise((resolve) => setTimeout(resolve, 10));
console.log(unhandled);
`);
  const ran = runSync([node!, join(dir, "run.mjs")], root, 30_000);
  expect(ran.stderr).toBe("");
  expect(ran.stdout).toBe('[7,"no",null]\n0\n');
});

// TypeScript's `then(onfulfilled)` gives what its handler does, a value or a
// promise of one, as `catch`'s does: react-dom's readAsDataURL returns its
// text from `blob.arrayBuffer().then(..)` (ADR 0782, amended).
test("a promise's then handler gives a value or a promise", async () => {
  const dir = fixture("js-promise-then-value");
  writeFileSync(join(dir, "lib.rs"), `use js::{Promise, promise};
pub fn texted(n: Promise<u32>) -> Promise<String> {
    n.then(|n| format!("n{n}"))
}
pub async fn chained() -> u32 {
    promise::resolve(1_u32).then(|n| promise::resolve(n + 1)).then(|n: u32| n * 10).await
}
`);
  const built = runSync([compiler, join(dir, "lib.rs"), "-o", join(dir, "lib.js"), "--", "--extern", `js=${join(target, "libjs.rmeta")}`, "-L", target], root, 600_000);
  expect(built.code, built.stderr).toBe(0);
  expect(built.stderr).not.toContain("warning");
  const js = withoutBoundary(readFileSync(join(dir, "lib.js"), "utf8"));
  expect(js).toContain("return n.then((n) => `n${n}`);");
  expect(js).toContain(".then((n) => Promise.resolve((n + 1) >>> 0))");
  writeFileSync(join(dir, "run.mjs"), `import * as lib from "./lib.js";
console.log(JSON.stringify([await lib.texted(Promise.resolve(4)), await lib.chained()]));
`);
  const ran = runSync([node!, join(dir, "run.mjs")], root, 30_000);
  expect(ran.stderr).toBe("");
  expect(ran.stdout).toBe('["n4",20]\n');
});

// React's lazy() stashes a promise's resolvers in its own `let`s, `new
// Promise((resolve, reject) => { resolveDebugValue = resolve; .. })`: the
// executor runs before `new Promise` returns, so it may borrow what's
// outside it (ADR 0782). And `then(onfulfilled, onrejected)`, TypeScript's
// two handlers, is `then_with`. Its amendment: a promise of a `Copy` value
// is `Copy`, a handle JS shares, kept in a `Cell` and read again,
// `ioInfo.value.then(..)`; one of any other value is consumed, rustc's
// E0382; and what rejects one takes any value, `rejectDebugValue(error)`
// of a `mixed` error, `Promise.reject()` of none.
test("a promise's executor may set the variables around it, and then takes two handlers", async () => {
  const dir = fixture("js-promise-executor");
  writeFileSync(join(dir, "lib.rs"), `use std::cell::Cell;

use js::{JsError, Promise, Unknown, error, js_error, promise};

pub struct Debug {
    pub resolve: &'static dyn Fn(u32),
    pub reject: &'static dyn Fn(Option<&Unknown>),
    pub value: Promise<u32>,
}

pub fn stash() -> Debug {
    let mut resolve_debug_value: Option<&'static dyn Fn(u32)> = None;
    let mut reject_debug_value: Option<&'static dyn Fn(Option<&Unknown>)> = None;
    let value = promise::new(|resolve, reject| {
        resolve_debug_value = Some(resolve);
        reject_debug_value = Some(reject);
    });
    Debug {
        resolve: resolve_debug_value.unwrap(),
        reject: reject_debug_value.unwrap(),
        value,
    }
}

pub async fn settled(fail: bool) -> String {
    let value = match fail {
        true => promise::reject::<u32>(Some(error::new(Some("no"), None))),
        false => promise::resolve(7),
    };
    value.then_with(|n| n.to_string(), |e| e.message()).await
}

pub fn hidden(value: Promise<u32>) {
    let _: Promise<()> = value.then_with(|_| {}, |_| {});
}

pub struct Info {
    #[cfg_attr(rust_js, rust_js::nullable)]
    pub value: Cell<Option<Promise<Option<&'static Unknown>>>>,
}

pub fn rejected(info: &Info, error: Option<&'static Unknown>) -> Promise<String> {
    let mut reject_debug_value: Option<&'static dyn Fn(Option<&Unknown>)> = None;
    info.value.set(Some(promise::new(|_resolve, reject| {
        reject_debug_value = Some(reject);
    })));
    let _: Promise<()> = info.value.get().unwrap().then_with(|_| {}, |_| {});
    reject_debug_value.unwrap()(error);
    let shown = info.value.get().unwrap();
    shown.then_with(|_| String::from("fulfilled"), |e| js_error::to_string(e))
}

pub fn twice(value: Promise<u32>) -> Promise<u32> {
    let _: Promise<()> = value.then_with(|_| {}, |_| {});
    value.then(|n| n + 1)
}

pub fn rethrown(error: &JsError) -> Promise<u32> {
    promise::reject(Some(error))
}

pub fn of_nothing() -> Promise<u32> {
    promise::reject(None)
}
`);
  const built = runSync([compiler, join(dir, "lib.rs"), "-o", join(dir, "lib.js"), "--", "--extern", `js=${join(target, "libjs.rmeta")}`, "-L", target], root, 600_000);
  expect(built.code, built.stderr).toBe(0);
  expect(built.stderr).not.toContain("warning");
  const js = withoutBoundary(readFileSync(join(dir, "lib.js"), "utf8"));
  expect(js).toContain(`  const value = new Promise((resolve, reject) => {
    resolveDebugValue = resolve;
    rejectDebugValue = reject;
  });`);
  expect(js).toContain("return await value.then(\n    (n) => String(n),\n    (e) => e.message,\n  );");
  expect(js).toContain("  value.then(\n    () => {},\n    () => {},\n  );");
  expect(js).toContain("  $unwrap(rejectDebugValue, \"lib.rs:48:24\")(error);\n");
  expect(js).toContain("  return value.then((n) => (n + 1) >>> 0);\n");
  expect(js).toContain("  return Promise.reject(error);\n");
  expect(js).toContain("  return Promise.reject();\n");
  writeFileSync(join(dir, "run.mjs"), `import * as lib from "./lib.js";
let unhandled = 0;
process.on("unhandledRejection", () => unhandled++);
const debug = lib.stash();
debug.resolve(3);
console.log(await debug.value, await lib.settled(false), await lib.settled(true));
lib.hidden(Promise.reject(new Error("hidden")));
const info = {};
console.log(await lib.rejected(info, "lost"), await lib.rejected({}, undefined), info.value instanceof Promise);
console.log(await lib.twice(Promise.resolve(1)));
console.log(await lib.rethrown(new TypeError("again")).catch(String), await lib.ofNothing().catch((e) => e));
await new Promise((resolve) => setTimeout(resolve, 10));
console.log(unhandled);
`);
  const ran = runSync([node!, join(dir, "run.mjs")], root, 30_000);
  expect(ran.stderr).toBe("");
  expect(ran.stdout).toBe("3 7 no\nlost undefined true\n2\nTypeError: again undefined\n0\n");
  // A promise of a value that isn't `Copy` is consumed: two handlers given
  // the one array it fulfils with would each change it.
  writeFileSync(join(dir, "moved.rs"), `use js::Promise;

pub fn both(value: Promise<Vec<u32>>) -> Promise<usize> {
    let _: Promise<()> = value.then(|mut v| v.push(1));
    value.then(|v| v.len())
}
`);
  const moved = runSync([compiler, join(dir, "moved.rs"), "-o", join(dir, "moved.js"), "--", "--extern", `js=${join(target, "libjs.rmeta")}`, "-L", target], root, 600_000);
  expect(moved.code).not.toBe(0);
  expect(moved.stderr).toContain("E0382");
});

// rd621, ADR 0317 amended: React's Flight client's DEV `then` makes its
// callbacks in the executor of the promise they settle, and they write on
// that promise, which they read through a cell set right after `new
// Promise` returns. The executor only stores them in the maker's `let`s,
// read after the cell is set, so none can run before: the cell is the
// promise's `const`, which they close over, as upstream's do. The executor
// may settle the promise itself at once, `res(..)`, which is no reader.
test("callbacks made in a promise's executor read the promise's const", async () => {
  const dir = fixture("js-promise-wrapper");
  writeFileSync(join(dir, "lib.rs"), `use std::cell::{Cell, RefCell};
use std::rc::Rc;

use js::{Promise, Unknown, promise};

pub struct Chunk {
    pub debug_info: RefCell<Option<&'static Unknown>>,
}

pub fn wrap(owner: &Rc<Chunk>, settled: bool) -> Promise<u32> {
    let mut resolve: Option<Rc<dyn Fn(u32)>> = None;
    let mut reject: Option<Rc<dyn Fn(Option<&'static Unknown>)>> = None;
    let wrapper: Rc<Cell<Option<Promise<u32>>>> = Rc::default();
    let wrapper_promise: Promise<u32> = promise::new(|res, rej| {
        let (chunk, resolved) = (Rc::clone(owner), Rc::clone(&wrapper));
        resolve = Some(Rc::new(move |value| {
            js::set(resolved.get().unwrap(), "_debugInfo", *chunk.debug_info.borrow());
            res(value);
        }));
        let (chunk, rejected) = (Rc::clone(owner), Rc::clone(&wrapper));
        reject = Some(Rc::new(move |reason| {
            js::set(rejected.get().unwrap(), "_debugInfo", *chunk.debug_info.borrow());
            rej(reason);
        }));
        if settled {
            res(1);
        }
    });
    wrapper.set(Some(wrapper_promise));
    match (resolve, reject) {
        (Some(resolve), _) if !settled => resolve(2),
        (_, Some(reject)) if settled => reject(None),
        _ => {}
    }
    wrapper_promise
}
`);
  const built = runSync([compiler, join(dir, "lib.rs"), "-o", join(dir, "lib.js"), "--", "--extern", `js=${join(target, "libjs.rmeta")}`, "-L", target], root, 600_000);
  expect(built.code, built.stderr).toBe(0);
  const js = withoutBoundary(readFileSync(join(dir, "lib.js"), "utf8"));
  expect(js).toContain(`  const wrapperPromise = new Promise((res, rej) => {
    resolve = (value) => {
      wrapperPromise._debugInfo = owner.debugInfo;
      res(value);
    };
    reject = (reason) => {
      wrapperPromise._debugInfo = owner.debugInfo;
      rej(reason);
    };`);
  expect(js).not.toContain("wrapper;");
  expect(js).not.toContain("$unwrap");
  writeFileSync(join(dir, "run.mjs"), `import * as lib from "./lib.js";
const resolved = lib.wrap({ debugInfo: "info" }, false);
const settled = lib.wrap({ debugInfo: "again" }, true);
console.log(await resolved, resolved._debugInfo, await settled, settled._debugInfo);
`);
  const ran = runSync([node!, join(dir, "run.mjs")], root, 30_000);
  expect(ran.stderr).toBe("");
  expect(ran.stdout).toBe("2 info 1 again\n");
});
