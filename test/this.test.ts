// `js::this()`, JS's `this` (rd683): React DOM's root server entries load
// their implementation where they're called, `require(..).renderToString
// .apply(this, arguments)`. Each call rust-js writes of a function that
// reads it gives none, as Rust's call gives none, a call through a
// module's namespace too; the function is only called, never a value,
// and no closure that forwards to it is it. Read where its JS's `this`
// isn't the one its JS caller gives, a compile error.

import { beforeAll, expect, test } from "bun:test";
import { mkdirSync, readFileSync, writeFileSync } from "node:fs";
import { join } from "node:path";
import { runSync } from "./child";
import { buildReact, compiler, fixture, run, target } from "./support";

beforeAll(buildReact, 600_000);

const js = ["--extern", `js=${join(target, "libjs.rmeta")}`, "-L", target];

const entries = `use js::{IntoUnknown, Unknown, function};

mod held {
    use js::Unknown;

    /// The test's function, which keeps what it's called with, its \`this\` first.
    #[cfg_attr(rust_js, rust_js::link_name = "get seen")]
    pub fn seen() -> &'static dyn Fn() -> Option<&'static Unknown> {
        unreachable!()
    }
}

/// Its \`this\` and what it's given, passed on as upstream's entry does.
pub fn render() -> Option<&'static Unknown> {
    function::apply(
        held::seen(),
        js::this().map(|this| this as &dyn IntoUnknown),
        Some(js::arguments()),
    )
}

/// Whether it was called on anything.
pub fn on_something() -> bool {
    js::this().is_some()
}

/// A Rust call gives none.
pub fn called() -> bool {
    on_something()
}

/// Through the module's namespace, none too, not the namespace.
pub fn called_through() -> bool {
    inner::on_something()
}

/// Through \`function::call\`, the \`this\` it's given.
pub fn called_on(value: &Unknown) -> Option<&'static Unknown> {
    function::call(&on_something, Some(value), ())
}

/// A closure that only calls it: an arrow, not \`on_something\`, which
/// JS would call on what holds it.
pub fn passing() -> Box<dyn Fn() -> bool> {
    Box::new(|| on_something())
}

pub mod inner {
    pub fn on_something() -> bool {
        js::this().is_some()
    }
}
`;

test("a function that reads its this is given the one JS calls it with, and none by Rust", async () => {
  const dir = fixture("this");
  writeFileSync(join(dir, "lib.rs"), entries);
  run([compiler, join(dir, "lib.rs"), "-o", join(dir, "lib.js"), "--", ...js]);
  const out = readFileSync(join(dir, "lib.js"), "utf8");
  expect(out).toContain("return seen.apply(this, arguments);");
  expect(out).toContain("return onSomething.call(value);");
  expect(out).toContain("return this != null;");
  expect(out).toContain("(0, Inner.onSomething)()");
  expect(out).toContain("() => onSomething()");
  const lib = await import(join(dir, "lib.js"));
  const calls: unknown[][] = [];
  (globalThis as Record<string, unknown>).seen = function (this: unknown, ...args: unknown[]) {
    calls.push([this, ...args]);
    return args.length;
  };
  // Each function by itself, as a plain call gives none, not \`lib\`'s namespace.
  const { render, onSomething, called, calledThrough, calledOn } = lib;
  const object = { render, onSomething, passing: lib.passing() };
  // Called as a method, plainly, and through `.apply` and `.call`.
  expect(object.render(1, 2)).toBe(2);
  expect(render(3)).toBe(1);
  expect(render.apply(object, [4, 5, 6])).toBe(3);
  expect(render.call("text", 7)).toBe(1);
  expect(render.call(null)).toBe(0);
  expect(calls).toEqual([[object, 1, 2], [undefined, 3], [object, 4, 5, 6], ["text", 7], [null]]);
  expect([object.onSomething(), onSomething(), onSomething.call(null), onSomething.call(0)]).toEqual([true, false, false, true]);
  // Rust's calls give none: by name, through the namespace and a forwarding
  // closure, each called on something.
  expect([lib.called(), lib.calledThrough(), object.passing()]).toEqual([false, false, false]);
  expect([called(), calledThrough(), calledOn(object), calledOn(0)]).toEqual([false, false, true, true]);
});

test("this read where its JS's isn't its JS caller's, or a function that reads it as a value, is refused", () => {
  const dir = fixture("this-refused");
  const refused = (body: string) => {
    writeFileSync(join(dir, "lib.rs"), `use js::Unknown;\n\n${body}`);
    const ran = runSync([compiler, join(dir, "lib.rs"), "-o", join(dir, "lib.js"), "--", ...js], dir, 120_000);
    expect(ran.code).not.toBe(0);
    return ran.stderr;
  };
  expect(refused("pub fn f() -> impl Fn() -> Option<&'static Unknown> {\n    || js::this()\n}\n"))
    .toContain("a closure is an arrow function, whose `this` is the function's around it");
  expect(refused("pub fn f() -> &'static dyn Fn() -> Option<&'static Unknown> {\n    js::function::with_this(|_, ()| js::this())\n}\n"))
    .toContain("a `function::with_this` closure's `this` is its first parameter");
  expect(refused("pub fn f<T>(value: T) -> T {\n    let _ = js::this();\n    value\n}\n"))
    .toContain("a generic function's JS is given its type parameters' dictionaries too");
  expect(refused("pub struct Point;\n\nimpl Point {\n    pub fn this(&self) -> Option<&'static Unknown> {\n        js::this()\n    }\n}\n"))
    .toContain("a method's `this` is its `self`");
  expect(refused("pub fn read() -> Option<&'static Unknown> {\n    js::this()\n}\n\npub fn held() -> fn() -> Option<&'static Unknown> {\n    read\n}\n"))
    .toContain("a function that reads `js::this()` as a value");
  expect(refused("pub fn held() -> Box<dyn Fn() -> Option<&'static Unknown>> {\n    Box::new(js::this)\n}\n"))
    .toContain("a function that reads `js::this()` as a value");
  // A binding of its own that's JS's \`this\`, which nothing checks where it's read.
  expect(refused("#[cfg_attr(rust_js, rust_js::link_name = \"this\")]\npub fn me() -> Option<&'static Unknown> {\n    unreachable!()\n}\n\npub fn f() -> bool {\n    me().is_some()\n}\n"))
    .toContain("does not support the `#[link_name]` of `me`");
});

// A library's function that reads it: its consumer's call gives none, the
// namespace's too, and its use as a value is refused, its contract's `this`.
test("a library's function that reads this is given none by its consumer", async () => {
  const dir = fixture("this-library");
  mkdirSync(join(dir, "dep"), { recursive: true });
  writeFileSync(join(dir, "dep.rs"), "pub mod entry {\n    pub fn on_something() -> bool {\n        js::this().is_some()\n    }\n}\n");
  run([compiler, join(dir, "dep.rs"), "-o", join(dir, "dep", "lib.js"), "--library", "--manifest", join(dir, "dep", "lib.manifest.json"),
    "--", "--crate-name", "dep", `--emit=metadata=${join(dir, "libdep.rmeta")}`, ...js]);
  const consumer = (source: string) => {
    mkdirSync(join(dir, "app"), { recursive: true });
    writeFileSync(join(dir, "app", "lib.rs"), source);
    return runSync([compiler, join(dir, "app", "lib.rs"), "-o", join(dir, "app", "lib.js"), "--dependency", join(dir, "dep", "lib.manifest.json"),
      "--", "--crate-name", "app", "--extern", `dep=${join(dir, "libdep.rmeta")}`, "-L", dir, ...js], dir, 120_000);
  };
  const ran = consumer("use dep::entry;\n\npub fn called() -> bool {\n    entry::on_something()\n}\n\npub fn passing() -> Box<dyn Fn() -> bool> {\n    Box::new(|| entry::on_something())\n}\n");
  expect(ran.stderr).toBe("");
  expect(ran.code).toBe(0);
  const app = readFileSync(join(dir, "app", "lib.js"), "utf8");
  expect(app).toContain("(0, Entry.onSomething)()");
  const lib = await import(join(dir, "app", "lib.js"));
  const object = { passing: lib.passing(), called: lib.called };
  expect([object.called(), object.passing()]).toEqual([false, false]);
  const refused = consumer("pub fn held() -> fn() -> bool {\n    dep::entry::on_something\n}\n");
  expect(refused.code).not.toBe(0);
  expect(refused.stderr).toContain("a function that reads `js::this()` as a value");
});
