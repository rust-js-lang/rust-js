// A scope guard that restores a thread-local `RefCell` no borrow of which
// is checked drops in a `finally` (#1930): alien-signals'
// `Finally(|| ACTIVE_SUB.set(prev_sub.clone()))`, `activeSub = prevSub`,
// upstream's `try { .. } finally { activeSub = prevSub; }`. Its `set` can't
// find the cell borrowed, the old `Rc` it drops runs nothing, and an `Rc`'s
// clone never unwinds. A guard whose closure panics keeps its abort-safe
// cleanup. Native Rust's run of the same program is the corpus case
// guard_restores_a_quiet_ref_cell.rs's.
import { beforeAll, expect, test } from "bun:test";
import { readFileSync, writeFileSync } from "node:fs";
import { join } from "node:path";
import { buildCompiler, compiler, fixture, run } from "./support";

beforeAll(buildCompiler, 600_000);

test("a guard restoring a RefCell no borrow of which is checked drops in a finally", async () => {
  const dir = fixture("guard-quiet-ref-cell");
  writeFileSync(
    join(dir, "lib.rs"),
    `use std::cell::RefCell;
use std::rc::Rc;

struct Finally<F: FnMut()>(F);

impl<F: FnMut()> Drop for Finally<F> {
    fn drop(&mut self) {
        (self.0)();
    }
}

pub struct Node {
    pub name: &'static str,
}

thread_local! {
    static ACTIVE: RefCell<Option<Rc<Node>>> = const { RefCell::new(None) };
}

pub fn active() -> Option<&'static str> {
    ACTIVE.with(|a| a.borrow().as_ref().map(|n| n.name))
}

pub fn quiet(work: impl Fn()) {
    let prev = ACTIVE.with(|a| a.borrow().clone());
    {
        let _restore = Finally(|| ACTIVE.set(prev.clone()));
        work();
    }
}

pub fn loud(work: impl Fn()) {
    let _restore = Finally(|| {
        if active().is_none() {
            panic!("none");
        }
    });
    work();
}

pub fn enter(name: &'static str) {
    ACTIVE.set(Some(Rc::new(Node { name })));
}
`,
  );
  run([compiler, join(dir, "lib.rs"), "-o", join(dir, "lib.js")]);
  const js = readFileSync(join(dir, "lib.js"), "utf8");
  const fn = (name: string) => js.slice(js.indexOf(`export function ${name}(`), js.indexOf("\n}\n", js.indexOf(`export function ${name}(`)) + 2);
  expect(fn("quiet")).toContain("finally");
  expect(fn("quiet")).not.toContain("$cleanup");
  expect(fn("loud")).toContain("$cleanup");
  const lib = await import(join(dir, "lib.js"));
  lib.enter("outer");
  lib.quiet(() => {
    lib.enter("inner");
    expect(lib.active()).toBe("inner");
  });
  expect(lib.active()).toBe("outer");
  expect(() => lib.quiet(() => {
    lib.enter("thrown");
    throw new Error("boom");
  })).toThrow("boom");
  expect(lib.active()).toBe("outer");
});
