// What the compiler writes of the js crate's objects given as a value of
// any shape, `js::unknown(..)`: each the object itself (ADR 0677,
// amended). Each test is a program of its own, compiled and run.

import { beforeAll, expect, test } from "bun:test";
import { readFileSync, writeFileSync } from "node:fs";
import { join } from "node:path";
import { buildReact, compiler, fixture, run, target } from "./support";

beforeAll(buildReact, 600_000);

// st1, React's throwException: `suspenseBoundary.updateQueue = new
// Set([wakeable]);`, an `any` field given a `Set`. Each of the js crate's
// objects is `js::Defined`, never `undefined` nor `null`, so a value of any
// shape, as a `Date` and a `JsObject` already were.
test("a Set, a Map, a Promise, an Error and a RegExp are each a value of any shape", async () => {
  const dir = fixture("objects-any-shape");
  writeFileSync(join(dir, "lib.rs"), `use std::cell::Cell;

use js::Unknown;

pub struct Fiber {
    pub update_queue: Cell<Option<&'static Unknown>>,
}

pub fn attach(fiber: &Fiber, wakeable: &'static Unknown) {
    fiber
        .update_queue
        .set(Some(js::unknown(js::set::new(Some(&[wakeable])))));
}

pub fn each(fiber: &Fiber, map: &'static js::Map<str, u32>, promise: &'static js::Promise<u32>) -> Vec<&'static Unknown> {
    let error = js::error::new("e", None);
    let pattern = js::reg_exp::new("a+", "g");
    let held = vec![js::unknown(map), js::unknown(promise), js::unknown(error), js::unknown(pattern)];
    fiber.update_queue.set(held.first().copied());
    held
}
`);
  run([compiler, join(dir, "lib.rs"), "-o", join(dir, "lib.js"), "--", "--extern", `js=${join(target, "libjs.rmeta")}`, "-L", target]);
  const js = readFileSync(join(dir, "lib.js"), "utf8");
  expect(js).toContain("  fiber.updateQueue = new Set([wakeable]);\n");
  const lib = await import(join(dir, "lib.js"));
  const fiber: { updateQueue: unknown } = { updateQueue: null };
  const wakeable = { then() {} };
  lib.attach(fiber, wakeable);
  expect(fiber.updateQueue).toBeInstanceOf(Set);
  expect([...(fiber.updateQueue as Set<unknown>)]).toEqual([wakeable]);
  const map = new Map([["a", 1]]);
  const promise = Promise.resolve(1);
  const [m, p, e, r] = lib.each(fiber, map, promise);
  expect([m, p]).toEqual([map, promise]);
  expect(fiber.updateQueue).toBe(map);
  expect(e).toBeInstanceOf(Error);
  expect((r as RegExp).source).toBe("a+");
});
