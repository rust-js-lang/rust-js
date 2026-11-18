// What a thread-local the crate alone reaches holds, its slot (#2114, ADR
// 1173): every write of it one of a closed list, so a value a builtin made
// kept in one is at hand read back, its methods the builtin's, and a borrow
// held across them plain. Anything else done with what it holds, a `pub`
// key, a JS caller's value written, its reference handed on, keeps it
// counted. Each program is compiled and run.

import { beforeAll, expect, test } from "bun:test";
import { readFileSync, writeFileSync } from "node:fs";
import { join } from "node:path";
import { buildReact, compiler, fixture, run, target } from "./support";

beforeAll(buildReact, 600_000);

async function compiled(name: string, source: string) {
  const dir = fixture(name);
  writeFileSync(join(dir, "lib.rs"), source);
  run([compiler, join(dir, "lib.rs"), "-o", join(dir, "lib.js"), "--", "--extern", `js=${join(target, "libjs.rmeta")}`, "-L", target]);
  return { js: readFileSync(join(dir, "lib.js"), "utf8"), lib: await import(join(dir, "lib.js")) };
}

// A memo whose maker asks `SEEN`, a slot of a set, under the cache's borrow;
// `seen` the slot's declaration, `more` what else the crate does with it.
const memo = (seen: string, more = "") => `use std::cell::RefCell;
use std::collections::HashMap;

thread_local! {
    static CACHE: RefCell<HashMap<String, f64>> = RefCell::new(HashMap::new());
    ${seen}
}
${more}
pub fn memo(key: &str) -> f64 {
    CACHE.with_borrow_mut(|cache| {
        *cache.entry(key.to_string()).or_insert_with(|| {
            SEEN.with_borrow_mut(|seen| seen.get_or_insert_with(|| js::set::new(None)).has(key) as u8 as f64)
        })
    })
}
`;

const checked = /\$withBorrowMut\(\s*cache,/;

test("a private thread-local's set a builtin made, read back, runs nothing", async () => {
  const { js, lib } = await compiled(
    "slot-private",
    memo("static SEEN: RefCell<Option<&'static js::Set<str>>> = RefCell::new(None);"),
  );
  expect(js).not.toMatch(checked);
  expect([lib.memo("a"), lib.memo("a")]).toEqual([0, 0]);
});

// Its cell lent whole, \`with\`'s, the thread-local isn't one the crate
// alone reaches (\`closed_keys\`): what it holds isn't taken at hand.
test("a thread-local whose cell is lent keeps the borrow across its set's methods checked", async () => {
  const { js } = await compiled(
    "slot-lent",
    memo(
      "static SEEN: RefCell<Option<&'static js::Set<str>>> = const { RefCell::new(None) };",
      `
pub fn made() -> bool {
    SEEN.with(|seen| seen.borrow().is_some())
}
`,
    ),
  );
  expect(js).toMatch(checked);
});

test("a thread-local a JS caller's set is written to keeps the borrow across its methods checked", async () => {
  const { js, lib } = await compiled(
    "slot-given",
    memo(
      "static SEEN: RefCell<Option<&'static js::Set<str>>> = const { RefCell::new(None) };",
      `
pub fn use_seen(seen: &'static js::Set<str>) {
    SEEN.set(Some(seen));
}
`,
    ),
  );
  expect(js).toMatch(checked);
  // A JS caller's set whose \`has\` calls the memo again meets its borrow.
  class Hand extends Set<string> {
    has(key: string) {
      lib.memo("again");
      return super.has(key);
    }
  }
  lib.useSeen(new Hand());
  expect(() => lib.memo("b")).toThrow(/already/);
});

test("a thread-local whose lent value is handed on keeps the borrow across its methods checked", async () => {
  const { js } = await compiled(
    "slot-handed-on",
    memo(
      "static SEEN: RefCell<Option<&'static js::Set<str>>> = const { RefCell::new(None) };",
      `
fn fill(seen: &mut Option<&'static js::Set<str>>) {
    seen.get_or_insert_with(|| js::set::new(None));
}

pub fn prepare() {
    SEEN.with_borrow_mut(|seen| fill(seen));
}
`,
    ),
  );
  expect(js).toMatch(checked);
});

// React's Fizz hooks' \`renderPhaseUpdates\`: a \`Cell\` of a map set where
// it's none, read by \`get\`.
test("a private Cell's map a builtin made, read back, runs nothing", async () => {
  const { js, lib } = await compiled(
    "slot-cell",
    `use std::cell::{Cell, RefCell};
use std::collections::HashMap;

thread_local! {
    static CACHE: RefCell<HashMap<String, f64>> = RefCell::new(HashMap::new());
    static COUNTS: Cell<Option<&'static js::Map<str, f64>>> = const { Cell::new(None) };
}

pub fn memo(key: &str) -> f64 {
    CACHE.with_borrow_mut(|cache| {
        *cache.entry(key.to_string()).or_insert_with(|| {
            let counts = match COUNTS.get() {
                Some(counts) => counts,
                None => {
                    let counts = js::map::new(None);
                    COUNTS.set(Some(counts));
                    counts
                }
            };
            counts.get(key).unwrap_or(0.0)
        })
    })
}
`,
  );
  expect(js).not.toMatch(checked);
  expect(lib.memo("a")).toBe(0);
});
