// The webapi crate's objects as JS has them: a named property, a weak
// collection's key. Each test is a program of its own, compiled and run.

import { beforeAll, expect, test } from "bun:test";
import { readFileSync, writeFileSync } from "node:fs";
import { join } from "node:path";

import { buildReact, compiler, fixture, run, target } from "./support";

beforeAll(buildReact, 600_000);

// React DOM's Fizz runtime reads and writes `element.dataset[name]`:
// DOMStringMap's IDL is all unnamed specials, a getter, a setter and a
// deleter of a name, each `this[name]` (#429).
test("a DOMStringMap's named properties are get, set and delete, dataset[name]", async () => {
  const withWeb = ["--", "--extern", `webapi=${join(target, "libwebapi.rmeta")}`, "--extern", `js=${join(target, "libjs.rmeta")}`, "-L", target];
  const dir = fixture("webapi-named-property");
  writeFileSync(join(dir, "lib.rs"), `use webapi::HTMLElement;

pub fn moved(node: &HTMLElement, from: &str, to: &str) -> Option<String> {
    let dataset = node.dataset();
    let value = dataset.get(from)?;
    dataset.set(to, &value);
    dataset.delete(from);
    Some(value)
}

pub fn cleared(node: &HTMLElement) -> impl Fn() + '_ {
    move || node.dataset().delete("x")
}
`);
  run([compiler, join(dir, "lib.rs"), "-o", join(dir, "lib.js"), ...withWeb]);
  const js = readFileSync(join(dir, "lib.js"), "utf8");
  expect(js).toContain("const value = dataset[from];");
  expect(js).toContain("dataset[to] = value;");
  expect(js).toContain("delete dataset[from];");
  const lib = await import(join(dir, "lib.js"));
  const dataset: Record<string, string> = { precedence: "high" };
  expect(lib.moved({ dataset }, "precedence", "p")).toBe("high");
  expect(dataset).toEqual({ p: "high" });
  expect(lib.moved({ dataset }, "missing", "q")).toBeUndefined();
  expect(dataset).toEqual({ p: "high" });
  // A closure of a delete gives what Rust's does, nothing: not `true`.
  expect(lib.cleared({ dataset: { x: "1" } })()).toBeUndefined();
});

// React DOM's Fizz config caches each Blob's data URL by the Blob,
// `WeakMap<Blob, Thenable<string>>`: every web API object is an object,
// TypeScript's `WeakKey`, which a weak collection holds (#446, ADR 0677).
test("a web API object is a WeakKey, a WeakMap's key", async () => {
  const withWeb = ["--", "--extern", `webapi=${join(target, "libwebapi.rmeta")}`, "--extern", `js=${join(target, "libjs.rmeta")}`, "-L", target];
  const dir = fixture("webapi-weak-key");
  writeFileSync(join(dir, "lib.rs"), `use std::cell::Cell;
use webapi::{Blob, Element};

thread_local! {
    static SIZES: Cell<Option<&'static js::WeakMap<Blob, f64>>> = const { Cell::new(None) };
}

pub fn size_of(blob: &'static Blob) -> f64 {
    let sizes = match SIZES.get() {
        Some(sizes) => sizes,
        None => {
            let sizes = js::weak_map::new();
            SIZES.set(Some(sizes));
            sizes
        }
    };
    if let Some(size) = sizes.get(blob) {
        return size;
    }
    let size = blob.size();
    sizes.set(blob, size);
    size
}

pub fn seen(element: &'static Element, set: &js::WeakSet<Element>) -> bool {
    let was = set.has(element);
    set.add(element);
    was
}
`);
  run([compiler, join(dir, "lib.rs"), "-o", join(dir, "lib.js"), ...withWeb]);
  const js = readFileSync(join(dir, "lib.js"), "utf8");
  expect(js).toContain("new WeakMap()");
  const lib = await import(join(dir, "lib.js"));
  const blob = { size: 3 };
  expect([lib.sizeOf(blob), lib.sizeOf(blob)]).toEqual([3, 3]);
  const set = new WeakSet();
  const element = {};
  expect([lib.seen(element, set), lib.seen(element, set)]).toEqual([false, true]);
});
