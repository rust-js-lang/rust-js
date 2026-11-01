// The node crate's `async_hooks`: a store each resource made in a `run`
// carries on, and a hook of each resource, as React's servers keep the
// request rendering, Fizz's `requestStorage`, and Flight's debug build
// tracks each promise awaited (rd492). Run in Node, whose hooks Bun lacks.

import { expect, test } from "bun:test";
import { readFileSync, writeFileSync } from "node:fs";
import { join } from "node:path";
import { runSync } from "./child";
import { buildNode, compiler, fixture, run } from "./support";

test("an AsyncLocalStorage's store is what each resource made in its run reads, and a hook sees each", () => {
  const dir = fixture("node-async-hooks");
  const node = buildNode();
  writeFileSync(join(dir, "lib.rs"), `use std::cell::Cell;
use std::rc::Rc;

use js::{JsObject, Unknown};
use node::async_hooks::{self, AsyncLocalStorage, HookCallbacks};

pub struct Request {
    pub id: f64,
}

thread_local! {
    static PROMISES: Cell<f64> = const { Cell::new(0.0) };
}

pub fn storage() -> &'static AsyncLocalStorage<Rc<Request>> {
    async_hooks::async_local_storage::new(None)
}

pub fn current(storage: &AsyncLocalStorage<Rc<Request>>) -> f64 {
    storage.get_store().map_or(-1.0, |request| request.id)
}

// The store is the storage's to keep, an \`Rc\`'s clone, as React's Fizz
// gives its request (rd497).
pub fn within(storage: &AsyncLocalStorage<Rc<Request>>, request: &Rc<Request>) -> f64 {
    storage.run(Rc::clone(request), || current(storage) * 10.0)
}

pub fn outside(storage: &AsyncLocalStorage<Rc<Request>>, request: &Rc<Request>) -> f64 {
    storage.run(Rc::clone(request), || storage.exit(|| current(storage)))
}

fn counted(_id: f64, kind: &str, _trigger: f64, _resource: &'static JsObject) {
    if kind == "PROMISE" {
        PROMISES.set(PROMISES.get() + 1.0);
    }
}

pub fn track() {
    async_hooks::create_hook(&HookCallbacks { init: Some(&counted), ..Default::default() }).enable();
}

pub fn promises() -> f64 {
    PROMISES.get()
}

pub fn ids() -> bool {
    async_hooks::execution_async_id() >= 0.0 && async_hooks::trigger_async_id() >= 0.0
}

pub fn id_of(value: &Unknown) -> Option<f64> {
    async_hooks::async_resource::async_id_of(value).ok().flatten()
}
`);
  run([compiler, join(dir, "lib.rs"), "-o", join(dir, "lib.js"), "--", "--extern", `node=${join(node, "libnode.rmeta")}`, "--extern", `js=${join(node, "libjs.rmeta")}`, "-L", node]);
  const js = readFileSync(join(dir, "lib.js"), "utf8");
  expect(js).toContain('import {\n  AsyncLocalStorage,\n  AsyncResource,\n  createHook,\n  executionAsyncId,\n  triggerAsyncId,\n} from "async_hooks";');
  expect(js).toContain("return new AsyncLocalStorage();");
  expect(js).toContain("return storage.run(request, () => current(storage) * 10);");
  expect(js).toContain("createHook({ init: counted }).enable();");
  expect(js).toContain("AsyncResource.prototype.asyncId.call(value)");
  writeFileSync(join(dir, "main.mjs"), `import * as lib from "./lib.js";
const storage = lib.storage();
const request = { id: 7 };
lib.track();
const before = lib.promises();
const later = await storage.run(request, () => Promise.resolve().then(() => lib.current(storage)));
console.log(JSON.stringify([lib.current(storage), lib.within(storage, request), lib.outside(storage, request), later, lib.promises() > before, lib.ids(), typeof lib.idOf(Promise.resolve()), lib.idOf(new Proxy({}, { get() { throw new Error("no"); } }))]));
`);
  const out = runSync(["node", join(dir, "main.mjs")], dir, 60_000);
  expect(out.stderr).toBe("");
  expect(JSON.parse(out.stdout)).toEqual([-1, 70, -1, 7, true, true, "number", null]);
});
