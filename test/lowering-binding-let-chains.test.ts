// A binding's property a let chain tests and binds, read once as Rust reads
// it, a getter's too (ADR 0672), and a thread-local of a let chain's value,
// its closure's const (ADR 0870). Split from lowering-bindings.test.ts by
// concept (#1334).

import { beforeAll, expect, test } from "bun:test";
import { readFileSync, writeFileSync } from "node:fs";
import { join } from "node:path";
import { buildReact, compiler, fixture, run, target } from "./support";

beforeAll(buildReact, 600_000);

// ADR 0672: a `get x` binding's read is one a getter may answer, as
// `js::get`'s is, which the binding's lowering says of the expression it
// writes (`js::Known::Getter`): a let chain that tests and binds it reads it
// once.
test("a get binding's property a let chain tests and binds is read once", async () => {
  const dir = fixture("get-binding-read-once");
  writeFileSync(join(dir, "lib.rs"), `#[cfg_attr(rust_js, rust_js::link_name = "get url")]
fn url(this: &js::JsObject) -> Option<String> {
    unreachable!()
}

pub fn long_url(page: &js::JsObject, ready: bool) -> String {
    if ready
        && let Some(found) = url(page)
        && found.len() > 3
    {
        return found;
    }
    String::new()
}
`);
  run([compiler, join(dir, "lib.rs"), "-o", join(dir, "lib.js"), "--", "--extern", `js=${join(target, "libjs.rmeta")}`, "-L", target]);
  const lib = await import(join(dir, "lib.js"));
  let reads = 0;
  const page = (url: unknown) => ({
    get url() {
      reads += 1;
      return url;
    },
  });
  expect(lib.longUrl(page("/api/x"), true)).toBe("/api/x");
  expect(lib.longUrl(page("/a"), true)).toBe("");
  expect(lib.longUrl(page("/api/x"), false)).toBe("");
  expect(reads).toBe(2);
});

// ADR 0672: `js::get(request, "url")` reads the property once, as Rust
// calls it once, so a getter runs once. Joined into the `if` before it,
// `request != null && typeof request.url === "string"`, the read a `let`
// chain binds was read again for its binding, `const url = request.url;`,
// where React's `getIODescription` tests `value.request.url`: a getter ran
// twice. A test reads in place what's bound before it only where that's read
// once, or reads nothing but variables.
test("a property a let chain tests and binds is read once", async () => {
  const dir = fixture("property-read-once");
  writeFileSync(join(dir, "lib.rs"), `use js::{Kind, Unknown};

pub fn requested_url(value: &Unknown) -> String {
    if let Some(request) = js::get(value, "request")
        && let Kind::String(url) = js::classify(js::get(request, "url"))
    {
        return url.to_string();
    }
    String::new()
}

pub fn has_url(value: &Unknown) -> bool {
    if let Some(request) = js::get(value, "request")
        && let Kind::String(_) = js::classify(js::get(request, "url"))
    {
        return true;
    }
    false
}

pub fn same(value: &Unknown, other: &Unknown) -> bool {
    if let Kind::String(a) = js::classify(js::get(value, "url"))
        && let Kind::String(b) = js::classify(js::get(other, "url"))
    {
        return a == b;
    }
    false
}

pub fn text_of(value: &Unknown) -> String {
    let text: String;
    if let Kind::String(own) = js::classify(Some(value)) {
        text = own.to_string();
    } else if let Kind::Object(object) = js::classify(Some(value))
        && let Some(inner) = js::get(object, "url")
        && let Kind::String(url) = js::classify(Some(inner))
    {
        text = url.to_string();
    } else {
        panic!("no text");
    }
    text
}
`);
  run([compiler, join(dir, "lib.rs"), "-o", join(dir, "lib.js"), "--", "--extern", `js=${join(target, "libjs.rmeta")}`, "-L", target]);
  const js = readFileSync(join(dir, "lib.js"), "utf8");
  expect(js).toContain(`export function requestedUrl(value) {
  const request = value.request;
  if (request != null) {
    const url = request.url;
    if (typeof url === "string") {
      return url;
    }
  }
  return "";
}`);
  // Read once, in the test, which binds nothing: in place.
  expect(js).toContain(`if (request != null && typeof request.url === "string") {
    return true;
  }`);
  expect(js.match(/request\.url/g)?.length).toBe(2);
  const lib = await import(join(dir, "lib.js"));
  const reads: string[] = [];
  const counted = (name: string, url: unknown) => ({
    get request() {
      reads.push(`${name}.request`);
      return {
        get url() {
          reads.push(`${name}.url`);
          return url;
        },
      };
    },
    get url() {
      reads.push(`${name}.url`);
      return url;
    },
  });
  expect(lib.requestedUrl(counted("a", "/api"))).toBe("/api");
  expect(lib.requestedUrl(counted("b", 7))).toBe("");
  expect(lib.requestedUrl({ request: null })).toBe("");
  expect(lib.hasUrl(counted("c", "/api"))).toBe(true);
  expect(lib.same(counted("d", "/x"), counted("e", "/x"))).toBe(true);
  expect(reads).toEqual(["a.request", "a.url", "b.request", "b.url", "c.request", "c.url", "d.url", "e.url"]);
  // Where what fails either test goes on alike, the read first, where the
  // first holds, and one test of it: no label to reach the `else`.
  expect(js).toContain(`  } else {
    const inner = typeof value === "object" && !Array.isArray(value) ? value.url : undefined;
    if (typeof inner === "string") {
      text = inner;
    } else {`);
  expect(js).not.toContain("break");
  reads.length = 0;
  expect([lib.textOf("own"), lib.textOf(counted("f", "/f"))]).toEqual(["own", "/f"]);
  expect(() => lib.textOf(counted("g", 7))).toThrow("no text");
  expect(() => lib.textOf(["x"])).toThrow("no text");
  expect(reads).toEqual(["f.url", "g.url"]);
});

// ADR 0870: React's `createTask`, a thread-local of a let chain's value,
// `if __DEV__ && let Some(create_task) = console::create_task() { .. } else
// { .. }`: the getter read once, where `__DEV__` holds, then a conditional
// of it, not a `let tmp` its branches set; both in the thread-local's own
// closure, its names its own (#407, ADR 0870 amended).
test("a thread-local of a let chain's value is its closure's const and value", async () => {
  const dir = fixture("chain-value-thread-local");
  writeFileSync(join(dir, "lib.rs"), `use webapi::{ConsoleTask, console};

const DEV: bool = true;

type CreateTask = dyn Fn(&str) -> Option<&'static ConsoleTask>;

#[cfg_attr(rust_js, rust_js::nullable)]
fn no_task(_name: &str) -> Option<&'static ConsoleTask> {
    None
}

thread_local! {
    static CREATE_TASK: Box<CreateTask> =
        // eslint-disable-next-line react-internal/no-production-logging
        if DEV && let Some(create_task) = console::create_task() {
            Box::new(move |name| Some(create_task(name)))
        } else {
            Box::new(no_task)
        };
}

pub fn task(name: &str) -> Option<&'static ConsoleTask> {
    CREATE_TASK.with(|create| create(name))
}
`);
  run([compiler, join(dir, "lib.rs"), "-o", join(dir, "lib.js"), "--", "--extern", `webapi=${join(target, "libwebapi.rmeta")}`, "--extern", `js=${join(target, "libjs.rmeta")}`, "-L", target]);
  const js = readFileSync(join(dir, "lib.js"), "utf8");
  expect(js).toContain(`const createTask = (() => {
  // eslint-disable-next-line react-internal/no-production-logging
  const createTask = DEV ? console.createTask : undefined;
  return createTask !== undefined ? (name) => createTask(name) : noTask;
})();
`);
  expect(js).not.toContain("tmp");
  const reads: string[] = [];
  const own = Object.getOwnPropertyDescriptor(console, "createTask");
  Object.defineProperty(console, "createTask", {
    configurable: true,
    get() {
      reads.push("createTask");
      return (name: string) => ({ name });
    },
  });
  try {
    const lib = await import(join(dir, "lib.js"));
    expect([lib.task("a"), lib.task("b")]).toEqual([{ name: "a" }, { name: "b" }]);
    expect(reads).toEqual(["createTask"]);
    // Where the host has none: `noTask`'s `null`.
    Object.defineProperty(console, "createTask", { configurable: true, value: undefined });
    const none = await import(`${join(dir, "lib.js")}?none`);
    expect(none.task("a")).toBeUndefined();
  } finally {
    if (own) Object.defineProperty(console, "createTask", own);
    else delete (console as { createTask?: unknown }).createTask;
  }
});

// #407 (ADR 0870 amended): a thread-local's `const` is its closure's, so a
// global of a binding's of its name that the module's other code reads is
// still the global, and neither is renamed.
test("a thread-local's const is its closure's, apart from a global of its name", async () => {
  const dir = fixture("chain-value-named-apart");
  writeFileSync(join(dir, "lib.rs"), `#[cfg_attr(rust_js, rust_js::link_name = "rustJsLabel")]
fn label() -> u32 {
    unreachable!()
}

const DEV: bool = true;

thread_local! {
    static FIRST: usize = if DEV && let Some(rust_js_label) = "ab".find('b') { rust_js_label } else { 0 };
}

pub fn labeled() -> u32 {
    label()
}

pub fn first() -> usize {
    FIRST.with(|first| *first)
}
`);
  run([compiler, join(dir, "lib.rs"), "-o", join(dir, "lib.js"), "--", "--extern", `js=${join(target, "libjs.rmeta")}`, "-L", target]);
  const js = readFileSync(join(dir, "lib.js"), "utf8");
  expect(js).toContain(`const FIRST = (() => {\n  const rustJsLabel = DEV ? $find("ab", "b") : undefined;\n  return rustJsLabel ?? 0;\n})();\n`);
  expect(js).toContain("  return rustJsLabel();\n");
  const globals = globalThis as { rustJsLabel?: () => number };
  globals.rustJsLabel = () => 7;
  try {
    const lib = await import(join(dir, "lib.js"));
    expect([lib.labeled(), lib.first()]).toEqual([7, 1]);
  } finally {
    delete globals.rustJsLabel;
  }
});
