// A `#[rust_js::nullable, rust_js::optional]` field, Flow's and
// TypeScript's `field?: T | null`, holds what's absent, `null` or a value
// (ADR 0810). Each test is a program of its own, compiled and run against
// objects JS makes, whose keys may be absent.

import { beforeAll, expect, test } from "bun:test";
import { readFileSync, writeFileSync } from "node:fs";
import { join } from "node:path";
import { buildReact, compiler, fixture, run, target } from "./support";

beforeAll(buildReact, 600_000);

// React's `processTransitionCallbacks`, which copies each abort's `name?:
// string | null` into a deletion's: an absent name reaches
// `onMarkerIncomplete` as `undefined`, as upstream's `name: abort.name`
// gives it, never `null`.
test("an optional nullable field given another's value stores it as it is", async () => {
  const dir = fixture("optional-copied");
  writeFileSync(join(dir, "lib.rs"), `pub struct Abort {
    pub reason: u8,
    #[cfg_attr(rust_js, rust_js::nullable, rust_js::optional)]
    pub name: Option<String>,
    #[cfg_attr(rust_js, rust_js::nullable, rust_js::optional)]
    pub label: Option<&'static str>,
}

pub struct Deletion {
    #[cfg_attr(rust_js, rust_js::nullable, rust_js::optional)]
    pub name: Option<String>,
    #[cfg_attr(rust_js, rust_js::nullable, rust_js::optional)]
    pub label: Option<&'static str>,
    pub end_time: f64,
}

pub struct Info {
    #[cfg_attr(rust_js, rust_js::nullable)]
    pub name: Option<String>,
}

pub fn deletions(aborts: &[Abort], end_time: f64) -> Vec<Deletion> {
    let mut filtered = Vec::new();
    aborts.iter().for_each(|abort| {
        filtered.push(Deletion {
            name: abort.name.clone(),
            label: abort.label,
            end_time,
        })
    });
    filtered
}

pub fn info(abort: &Abort) -> Info {
    Info {
        name: abort.name.clone(),
    }
}

pub fn rename(deletion: &mut Deletion, abort: &Abort) {
    deletion.name = abort.name.clone();
    deletion.label = abort.label;
}

pub fn named(deletion: &Deletion) -> bool {
    deletion.name.is_some()
}
`);
  run([compiler, "--declarations", join(dir, "lib.rs"), "-o", join(dir, "lib.js"), "--", "--extern", `js=${join(target, "libjs.rmeta")}`, "-L", target]);
  const js = readFileSync(join(dir, "lib.js"), "utf8");
  expect(js).toContain("filtered.push({\n      name: abort.name,\n      label: abort.label,\n      endTime,\n    });");
  expect(js).toContain("deletion.name = abort.name;\n  deletion.label = abort.label;");
  // A nullable field that isn't optional is `null` or a value: what may be absent is `?? null`.
  expect(js).toContain("name: abort.name ?? null,");
  const lib = await import(join(dir, "lib.js"));
  const aborts = [{ reason: 0 }, { reason: 1, name: null, label: null }, { reason: 2, name: "a", label: "b" }];
  const deletions = lib.deletions(aborts, 5);
  expect(deletions.map((d: Record<string, unknown>) => [d.name, d.label])).toEqual([
    [undefined, undefined],
    [null, null],
    ["a", "b"],
  ]);
  expect(deletions.map(lib.named)).toEqual([false, false, true]);
  expect(aborts.map((a) => lib.info(a).name)).toEqual([null, null, "a"]);
  const deletion = { name: "x", label: "y", endTime: 0 };
  lib.rename(deletion, aborts[0]);
  expect([deletion.name, deletion.label, lib.named(deletion)]).toEqual([undefined, undefined, false]);
});

// React's createFiberFromTypeAndProps copies `created._debugTask =
// returnFiber._debugTask;`, and cloneDependencies `_debugThenableState:
// d._debugThenableState`, each a `Cell` field of the port's: a cell's
// `get()` of an optional nullable field, given to another, is as it is, as
// a plain field's read is (ADR 0810, amended). One given to a nullable
// cell that isn't optional is `?? null`.
test("an optional nullable cell field given another's get stores it as it is", async () => {
  const dir = fixture("optional-cell-copied");
  writeFileSync(join(dir, "lib.rs"), `use std::cell::Cell;
use std::rc::Rc;

pub struct Fiber {
    pub tag: Cell<u32>,
    #[cfg_attr(rust_js, rust_js::nullable, rust_js::optional)]
    pub debug_task: Cell<Option<&'static str>>,
    #[cfg_attr(rust_js, rust_js::nullable)]
    pub task: Cell<Option<&'static str>>,
}

pub fn copy_task(created: &Fiber, return_fiber: &Fiber) {
    created.debug_task.set(return_fiber.debug_task.get());
    created.task.set(return_fiber.debug_task.get());
}

pub fn cloned(fiber: &Fiber) -> Rc<Fiber> {
    Rc::new(Fiber {
        tag: Cell::new(fiber.tag.get()),
        debug_task: Cell::new(fiber.debug_task.get()),
        task: Cell::new(fiber.debug_task.get()),
    })
}
`);
  run([compiler, join(dir, "lib.rs"), "-o", join(dir, "lib.js")]);
  const js = readFileSync(join(dir, "lib.js"), "utf8");
  expect(js).toContain("  created.debugTask = returnFiber.debugTask;\n  created.task = returnFiber.debugTask ?? null;\n");
  expect(js).toContain("    debugTask: fiber.debugTask,\n    task: fiber.debugTask ?? null,\n");
  const lib = await import(join(dir, "lib.js"));
  const created: Record<string, unknown> = { tag: 0, debugTask: "x", task: "y" };
  lib.copyTask(created, { tag: 1 });
  expect([created.debugTask, created.task]).toEqual([undefined, null]);
  const copy = lib.cloned({ tag: 2, debugTask: null, task: null });
  expect([copy.debugTask, copy.task]).toEqual([null, null]);
});
