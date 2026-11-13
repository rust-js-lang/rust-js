// What the compiler writes of async code: an awaited reference, a closure
// that only spawns, an async closure's parameters, what an async block
// drops, and a future's output a promise would adopt. Each test is a
// program of its own, compiled and run.

import { beforeAll, expect, test } from "bun:test";
import { readFileSync, writeFileSync } from "node:fs";
import { join } from "node:path";
import { buildReact, compiler, fixture, run, target } from "./support";

beforeAll(buildReact, 600_000);

// ADR 0029: an awaited reference given to a generic `&T` is awaited, though
// rustc reborrows it inside the `.await`.
test("an awaited reference given to a generic function is awaited", async () => {
  const dir = fixture("await-generic");
  writeFileSync(join(dir, "lib.rs"), `use js::Promise;

#[cfg_attr(rust_js, rust_js::link_name = "node:timers/promises#setTimeout")]
fn later(ms: u32, value: &str) -> Promise<&'static str> {
    unreachable!()
}

fn same<T: ?Sized>(x: &T) -> &T {
    x
}

pub async fn passed(text: &str) -> String {
    same(later(0, text).await).to_string()
}
`);
  run([compiler, join(dir, "lib.rs"), "-o", join(dir, "lib.js"), "--", "--extern", `js=${join(target, "libjs.rmeta")}`, "-L", target]);
  expect(readFileSync(join(dir, "lib.js"), "utf8")).toContain("return same(await setTimeout(0, text));");
  expect(await (await import(join(dir, "lib.js"))).passed("text")).toBe("text");
});

// A closure that only spawns an async block is an async arrow, as
// react.dev's BrandMenu writes `onSelect={async () => { await
// navigator.clipboard.writeText(..) }}`: it starts the same, and gives
// back a promise where nothing reads its `()` (ADR 0257). From MIR too
// (ADR 0364).
test("a closure that only spawns an async block is an async arrow", async () => {
  const dir = fixture("spawning-closure");
  writeFileSync(join(dir, "copy.js"), "export const copied = [];\nexport async function copy(text) { copied.push(text); }\n");
  writeFileSync(join(dir, "lib.rs"), `#[cfg_attr(rust_js, rust_js::link_name = "./copy.js#copy")] fn copy(text: &str) -> js::Promise<()> { unreachable!() }

pub fn handler() -> Box<dyn Fn()> {
    Box::new(|| {
        js::spawn(Box::new(async {
            copy("#58C4DC").await;
        }))
    })
}
`);
  const { copied } = await import(join(dir, "copy.js"));
  run([compiler, join(dir, "lib.rs"), "-o", join(dir, "mir.js"), "--", "--extern", `js=${join(target, "libjs.rmeta")}`, "-L", target]);
  const js = readFileSync(join(dir, "mir.js"), "utf8");
  expect(js).toContain('return async () => {\n    await copy("#58C4DC");\n  };');
  const lib = await import(join(dir, "mir.js"));
  copied.length = 0;
  await lib.handler()();
  expect(copied).toEqual(["#58C4DC"]);
});

// An `async` closure's `|_|`, which takes nothing it reads, takes nothing
// in JS either, `async () => ..`, as react.dev's PageHeading has its copy
// button's handler.
test("an async closure's ignored argument is no parameter", async () => {
  const dir = fixture("async-wild");
  writeFileSync(join(dir, "lib.rs"), `pub async fn run(f: impl AsyncFn(u32) -> u32) -> u32 {
    f(2).await
}
pub async fn made() -> u32 {
    run(async move |_| 1).await + run(async move |x| x).await
}
`);
  run([compiler, join(dir, "lib.rs"), "-o", join(dir, "mir.js")]);
  const js = readFileSync(join(dir, "mir.js"), "utf8");
  expect(js).toContain("run(async () => 1)");
  const lib = await import(join(dir, "mir.js"));
  expect(await lib.made()).toBe(3);
});

// #173: an \`async move\` block drops what it took as its body ends, where
// Rust's future, awaited, is dropped; one dropped unawaited, whose drop may
// be seen, is refused, as it would have run, a promise (#965, ADR 0029,
// amended). An \`async\` closure drops what it took where it's dropped, as
// a closure does; an \`async fn\` its arguments, once.
test("an async block or closure drops what it took", async () => {
  const dir = fixture("async-drops");
  writeFileSync(join(dir, "lib.rs"), `pub struct D(pub &'static str, pub std::rc::Rc<std::cell::RefCell<Vec<String>>>);
impl Drop for D {
    fn drop(&mut self) {
        self.1.borrow_mut().push(format!("drop {}", self.0));
    }
}
async fn consume(d: D) {
    d.1.borrow_mut().push(format!("consume {}", d.0));
}
pub async fn report() -> Vec<String> {
    let log = std::rc::Rc::new(std::cell::RefCell::new(Vec::new()));
    let d = D("d", log.clone());
    async move {
        d.1.borrow_mut().push(format!("in {}", d.0));
    }
    .await;
    log.borrow_mut().push("after".to_string());
    let c = D("c", log.clone());
    let call = async move || {
        c.1.borrow_mut().push(format!("call {}", c.0));
    };
    call().await;
    call().await;
    drop(call);
    let e = D("e", log.clone());
    let once = async move || {
        let taken = e;
        taken.1.borrow_mut().push(format!("took {}", taken.0));
    };
    once().await;
    consume(D("f", log.clone())).await;
    log.borrow_mut().push("end".to_string());
    let lines = log.borrow().clone();
    lines
}
`);
  run([compiler, join(dir, "lib.rs"), "-o", join(dir, "lib.js")]);
  const { report } = await import(join(dir, "lib.js"));
  expect(await report()).toEqual(["in d", "drop d", "after", "call c", "call c", "drop c", "took e", "drop e", "consume f", "drop f", "end"]);
});

// A future's output a promise would adopt, a thenable, is boxed where the
// promise is resolved, `$settled`, and taken out where it's awaited,
// `$unsettled` (#172): a struct with a function named `then`, and a future
// of a future, which a promise would flatten. A future's output that can't
// be one is as it is.
test("an awaited value that's a thenable isn't adopted", async () => {
  const dir = fixture("async-thenable");
  writeFileSync(join(dir, "lib.rs"), `use std::pin::Pin;
pub struct Hook {
    then: Box<dyn Fn(u32) -> u32>,
}
async fn make(n: u32) -> Hook {
    Hook { then: Box::new(move |x| x + n) }
}
async fn pick() -> Option<Pin<Box<dyn Future<Output = ()>>>> {
    Some(Box::pin(async {}))
}
async fn plain() -> u32 {
    4
}
pub async fn report() -> String {
    let hook = make(5).await;
    let ready = std::future::ready(Hook { then: Box::new(|x| x * 3) }).await;
    let inner = pick().await.is_some();
    format!("{} {} {} {}", (hook.then)(1), (ready.then)(2), inner, plain().await)
}
`);
  run([compiler, join(dir, "lib.rs"), "-o", join(dir, "lib.js")]);
  const js = readFileSync(join(dir, "lib.js"), "utf8");
  expect(js).toContain("await plain()}");
  expect(js).toContain("$unsettled(await make(5))");
  // One adopted never resolves: a second on, it's taken for hung.
  const hung = new Promise((resolve) => setTimeout(() => resolve("hung"), 1000));
  expect(await Promise.race([(await import(join(dir, "lib.js"))).report(), hung])).toBe("6 6 true 4");
});
