// What an exit that gives `None` as `undefined` alone gives as it is, by
// MIR (#2112, Change C's third step, `gives_no_null`): a call's write of
// `js::null()`, JS's `null` on purpose (ADR 0935), beside a binding's
// `null`; `js::null()`s given to a binding's optional arguments; a `Some`
// tested there; and a `Some` of what JS's `new` made, through a JS object's
// `Deref` (ADR 0024). `?`'s `None` returned, Rust's own, is
// test/snapshots/throws' `return;`. A `Some` of what JS threw, a caught
// `null` (ADR 0030), and `exec`'s no match are Rust's `None`.

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

test("js::null() returned beside a binding's null is JS's null, as it is", async () => {
  const { js, lib } = await compiled(
    "one-none-js-null",
    `pub fn found(text: &str) -> Option<&'static js::RegExpMatch> {
    if text.is_empty() {
        return js::null();
    }
    js::reg_exp::new("^a", "").exec(text)
}
`,
  );
  expect(js).toContain("return null;");
  expect(lib.found("")).toBeNull();
  expect(lib.found("b")).toBeUndefined();
  expect(lib.found("ab")[0]).toBe("a");
});

// React's Flight server's `busboyStream.destroy(Some(js::error::new(..)))`:
// a binding's optional argument of a `Some` of what JS's `new` made, through
// `Error`'s `Deref` to `JsError` (ADR 0024), is given as it is; of what JS
// threw, a caught `null` (ADR 0030), Rust's `None` where it's `null`. Its
// module isn't there, so its JS is read, not run.
const destroyed = (body: string) => `#[cfg_attr(rust_js, rust_js::link_name = "./stream.js#destroy")]
fn destroy(error: Option<&js::JsError>) {
    let _ = error;
    unreachable!()
}

${body}
`;

function read(name: string, source: string) {
  const dir = fixture(name);
  writeFileSync(join(dir, "lib.rs"), source);
  run([compiler, join(dir, "lib.rs"), "-o", join(dir, "lib.js"), "--", "--extern", `js=${join(target, "libjs.rmeta")}`, "-L", target]);
  return readFileSync(join(dir, "lib.js"), "utf8");
}

test("a Some of an error new made, given to a binding's optional argument, is as it is", () => {
  const js = read(
    "one-none-new",
    destroyed(`pub fn failed(message: &str) {
    destroy(Some::<&js::JsError>(js::error::new(Some(message), None)));
}`),
  );
  expect(js).toContain("destroy(new Error(message))");
});

test("a Some of what JS threw, given to a binding's optional argument, is Rust's None where it's null", () => {
  const js = read(
    "one-none-thrown",
    destroyed(`pub fn failed(source: &str) {
    if let Err(error) = js::eval(source) {
        destroy(Some(error));
    }
}`),
  );
  expect(js).toContain("error != null ? destroy(error) : destroy()");
});

test("a Some of what JS threw, a caught null, is Rust's None", async () => {
  const { js, lib } = await compiled(
    "one-none-caught",
    `pub fn caught(source: &str) -> Option<&'static js::JsError> {
    match js::eval(source) {
        Err(error) => Some(error),
        Ok(_) => None,
    }
}
`,
  );
  expect(js).toContain("?? undefined");
  expect(lib.caught("throw 1")).toBe(1);
  expect(lib.caught("throw null")).toBeUndefined();
  expect(lib.caught("1")).toBeUndefined();
});

test("exec's no match is Rust's None", async () => {
  const { js, lib } = await compiled(
    "one-none-exec",
    `pub fn matched(text: &str) -> Option<&'static js::RegExpMatch> {
    js::reg_exp::new("^a", "").exec(text)
}
`,
  );
  expect(js).toContain("?? undefined");
  expect(lib.matched("b")).toBeUndefined();
  expect(lib.matched("ab")[0]).toBe("a");
});

// React's Fizz server's `writeStartClientRenderedSuspenseBoundary(..,
// errorDigest, js::null(), ..)`: a binding's optional arguments, each
// `js::null()` JS's `null` on purpose (ADR 0935), given as they are, never
// left out as a `None` would be. Its module isn't there, so its JS is read,
// not run.
test("js::null() given to a binding's optional arguments is JS's null, as it is", () => {
  const dir = fixture("one-none-written-null-args");
  writeFileSync(
    join(dir, "lib.rs"),
    `use std::cell::RefCell;

#[cfg_attr(rust_js, rust_js::link_name = "./config.js#startBoundary")]
fn start_boundary(digest: Option<&str>, message: Option<&str>, stack: Option<&str>) -> bool {
    let _ = (digest, message, stack);
    unreachable!()
}

pub struct Boundary {
    digest: RefCell<Option<String>>,
}

pub fn set_digest(boundary: &Boundary, digest: Option<String>) {
    *boundary.digest.borrow_mut() = digest;
}

pub fn flushed(boundary: &Boundary) -> bool {
    let digest = boundary.digest.borrow().clone();
    start_boundary(digest.as_deref(), js::null(), js::null())
}
`,
  );
  run([compiler, join(dir, "lib.rs"), "-o", join(dir, "lib.js"), "--", "--extern", `js=${join(target, "libjs.rmeta")}`, "-L", target]);
  expect(readFileSync(join(dir, "lib.js"), "utf8")).toContain("startBoundary(digest ?? undefined, null, null)");
});

// React DOM's scroll-end polyfill's `clearTimeout(existingTimer)` under
// `existing_timer.is_some()`: what a test proved `Some` there, of a payload
// never JS's `null`, is given to a binding's optional argument as it is.
test("a tested Some given to a binding's optional argument is as it is", async () => {
  const { js, lib } = await compiled(
    "one-none-proved",
    `pub fn cancel(timer: Option<&js::TimeoutId>) -> bool {
    if timer.is_some() {
        js::clear_timeout(timer);
        return true;
    }
    false
}
`,
  );
  expect(js).toContain("clearTimeout(timer);");
  expect([lib.cancel(setTimeout(() => {}, 1e6)), lib.cancel(null), lib.cancel(undefined)]).toEqual([true, false, false]);
});
