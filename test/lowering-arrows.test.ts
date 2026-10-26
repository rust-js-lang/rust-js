// What the compiler writes of a closure's body: `() => f()` of one the
// program wrote as its call's value, where that call's JS gives
// `undefined`, as Rust's closure gives `()`; a block of one written as a
// statement, or of a call whose JS gives something else. Each test is a
// program of its own, compiled and run.

import { expect, test } from "bun:test";
import { readFileSync, writeFileSync } from "node:fs";
import { join } from "node:path";
import { buildNode, compiler, fixture, run, target } from "./support";

// ADR 0883: a closure the program wrote as its call's value, `|| f()`, is
// `() => f()` where the call's JS gives `undefined`: a function of the
// crate's that gives `()`, a closure of its own, or a binding that says so,
// a WebIDL operation of `undefined`. What JS reads of it, `f.call()`'s
// value, is `undefined`, as Rust's `()` is. Of a call whose JS gives
// something else, `Array.prototype.push`'s count, it's a block; and one
// written `|| { f(); }` is a block, as written.
test("a closure of its call's value is that value where it's undefined", async () => {
  const node = buildNode();
  const dir = fixture("arrow-values");
  writeFileSync(join(dir, "lib.rs"), `use js::function;
use std::cell::RefCell;

fn log(seen: &RefCell<Vec<u32>>, n: u32) {
    seen.borrow_mut().push(n);
}

fn read(f: &dyn Fn()) -> bool {
    function::call(f, None, ()).is_none()
}

pub fn values() -> Vec<bool> {
    let seen = RefCell::new(Vec::new());
    let crates = || log(&seen, 1);
    let statement = || {
        log(&seen, 2);
    };
    let pushed = || seen.borrow_mut().push(3);
    let web = || webapi::console::log(("logged",));
    vec![read(&crates), read(&statement), read(&pushed), read(&web), seen.borrow().len() == 3]
}
`);
  const crates = ["js", "node", "webapi"].flatMap((lib) => ["--extern", `${lib}=${join(node, `lib${lib}.rmeta`)}`]);
  run([compiler, join(dir, "lib.rs"), "-o", join(dir, "lib.js"), "--", ...crates, "-L", node]);
  const js = readFileSync(join(dir, "lib.js"), "utf8");
  expect(js).toContain("const crates = () => log(seen, 1);");
  // One of statements a function declared, as React's factories write
  // theirs (ADR 0882, amended).
  expect(js).toContain("function statement() {\n    log(seen, 2);\n  }");
  expect(js).toContain("function pushed() {\n");
  expect(js).toContain('const web = () => console.log("logged");');
  const logged: unknown[] = [];
  const previous = console.log;
  console.log = (...args: unknown[]) => {
    logged.push(args);
  };
  try {
    expect((await import(join(dir, "lib.js"))).values()).toEqual([true, true, true, true, true]);
  } finally {
    console.log = previous;
  }
  expect(logged).toEqual([["logged"]]);
});

// ADR 0882, amended (the user's decision): React's ChildFiber factory,
// `function deleteChild(returnFiber, childToDelete) { .. }`: a closure of
// statements a `let` binds is a function declared, an async one's an
// `async function`; one of a value alone stays an arrow, and so does one
// that reads a class's `this`, which a function has of its own.
test("a let's closure of statements is a function declared", async () => {
  const dir = fixture("let-closure-functions");
  writeFileSync(
    join(dir, "lib.rs"),
    `pub fn make(track: bool) -> Box<dyn Fn(u32) -> u32> {
    let delete_child = move |n: u32| {
        if !track {
            return n;
        }
        n + 1
    };
    let double = |n: u32| n * 2;
    let log = move |n: u32| {
        let m = double(n);
        js::console_log(m);
    };
    Box::new(move |n| {
        log(n);
        delete_child(n)
    })
}

pub async fn later(n: u32) -> u32 {
    let step = async move |m: u32| {
        let k = m + n;
        k * 2
    };
    step(1).await
}

#[cfg_attr(rust_js, rust_js::class)]
pub struct Counter {
    pub n: u32,
}

#[cfg_attr(rust_js, rust_js::class)]
impl Counter {
    pub fn new(n: u32) -> Self {
        Counter { n }
    }

    pub fn twice(&self, k: u32) -> u32 {
        let read = |k: u32| {
            let x = self.n + k;
            x * 2
        };
        read(k) + read(k)
    }
}
`.replace("js::console_log(m);", "let _ = m;"),
  );
  run([compiler, join(dir, "lib.rs"), "-o", join(dir, "lib.js"), "--", "--extern", `js=${join(target, "libjs.rmeta")}`, "-L", target]);
  const js = readFileSync(join(dir, "lib.js"), "utf8");
  expect(js).toContain("  function deleteChild(n) {\n    if (!track) {\n      return n;\n    }\n");
  expect(js).toContain("  const double = (n) => (n * 2) >>> 0;\n");
  expect(js).toContain("  function log(n) {\n");
  expect(js).toContain("  async function step(m) {\n");
  expect(js).toContain("  const read = (k) => {\n    const x = (this.n + k) >>> 0;\n");
  const lib = await import(join(dir, "lib.js"));
  expect([lib.make(true)(1), lib.make(false)(1), await lib.later(2), new lib.Counter(4).twice(1)]).toEqual([2, 1, 6, 20]);
});

// ADR 0882, amended: a comment before a statement of a function declared
// in a body is before that statement, not inside a closure it's given.
test("a comment in a let's function declared is where it's written", async () => {
  const dir = fixture("let-closure-function-comment");
  writeFileSync(
    join(dir, "lib.rs"),
    `pub fn run(items: &[u32]) -> u32 {
    let mut total = 0;
    let mut step = |n: u32| {
        let doubled = n * 2;
        // Each item, kept.
        items.iter().for_each(|&m| {
            total += m + doubled;
        });
    };
    step(1);
    step(2);
    total
}
`,
  );
  run([compiler, join(dir, "lib.rs"), "-o", join(dir, "lib.js")]);
  const js = readFileSync(join(dir, "lib.js"), "utf8");
  expect(js).toContain("    const doubled = (n * 2) >>> 0;\n    // Each item, kept.\n    items.forEach((m) => {\n");
  const lib = await import(join(dir, "lib.js"));
  expect(lib.run([1, 2])).toBe(3 + 2 + 2 + 3 + 4 + 4);
});
