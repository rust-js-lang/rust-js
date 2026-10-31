// A module's namespace as a value (rd292, ADR 1037): a unit struct and its
// impl marked `#[rust_js::namespace]` are the module they're in, its `&self`
// methods the module's exported functions, its value `import * as M`. React's
// noop renderer is given the reconciler's namespace, `NoopRenderer =
// reconciler(hostConfig)`, and calls through it, `NoopRenderer.createContainer
// (..)`; React DOM, which imports the module, calls `createContainer(..)`.
// Each test is a program of its own, compiled and run in Node.

import { beforeAll, expect, test } from "bun:test";
import { readFileSync, writeFileSync } from "node:fs";
import { join } from "node:path";
import { runSync } from "./child";
import { buildReact, compiler, fixture, root, run, target } from "./support";

beforeAll(buildReact, 600_000);

const flags = (dir: string) => [
  "--declarations",
  join(dir, "lib.rs"),
  "-o",
  join(dir, "lib.js"),
  "--",
  "--extern",
  `js=${join(target, "libjs.rmeta")}`,
  "-L",
  target,
];

const reconciler = `use std::cell::Cell;

thread_local! {
    static ROOTS: Cell<u32> = const { Cell::new(0) };
}

/// The reconciler's API, this module.
#[cfg_attr(rust_js, rust_js::namespace)]
pub struct Reconciler;

unsafe impl js::Defined for Reconciler {}

#[cfg_attr(rust_js, rust_js::namespace)]
impl Reconciler {
    /// Makes a root of \`tag\`.
    pub fn create_container(&self, tag: u32) -> u32 {
        ROOTS.set(ROOTS.get() + 1);
        self.update_container(tag) + 1
    }

    pub fn update_container(&self, n: u32) -> u32 {
        n * 2
    }
}

pub fn roots() -> u32 {
    ROOTS.get()
}

pub fn me() -> &'static Reconciler {
    &Reconciler
}
`;

/** The program of a reconciler module and \`lib\`, compiled: its JS files, declarations, and where they are. */
function compiled(
  name: string,
  lib: string,
): { js: string; module: string; ts: string; dir: string } {
  const dir = fixture(name);
  writeFileSync(join(dir, "lib.rs"), lib);
  writeFileSync(join(dir, "reconciler.rs"), reconciler);
  run([compiler, ...flags(dir)]);
  const read = (file: string) => readFileSync(join(dir, file), "utf8");
  return {
    js: read("lib.js"),
    module: read("reconciler.js"),
    ts: read("lib.d.ts"),
    dir,
  };
}

test("a namespace's methods are its module's functions, called through a value or by name", async () => {
  const { js, module, ts, dir } = compiled(
    "namespace-methods",
    `pub mod reconciler;

use js::{JsObject, object};
use reconciler::Reconciler;

pub fn direct() -> u32 {
    Reconciler.create_container(3)
}

pub fn through(renderer: &Reconciler) -> u32 {
    renderer.create_container(4) + renderer.update_container(1)
}

pub fn namespace() -> &'static Reconciler {
    &Reconciler
}

pub fn config() -> &'static JsObject {
    object::spread(Some(js::unknown(&Reconciler)))
}

pub fn method() -> fn(&Reconciler, u32) -> u32 {
    Reconciler::update_container
}
`,
  );
  // Its functions are the module's, exported, `self` no parameter, a call
  // through it of its own a call of the function.
  expect(module).toContain(
    "/** Makes a root of `tag`. */\nexport function createContainer(tag) {\n",
  );
  expect(module).toContain("  return (updateContainer(tag) + 1) >>> 0;\n");
  expect(module).toContain("export function updateContainer(n) {\n");
  // Named, the function itself; through a value, the value's member.
  expect(js).toContain('import { createContainer } from "./reconciler.js";\n');
  expect(js).toContain('import * as Reconciler from "./reconciler.js";\n');
  expect(js).toContain("  return createContainer(3);\n");
  expect(js).toContain(
    "  return (renderer.createContainer(4) + renderer.updateContainer(1)) >>> 0;\n",
  );
  expect(js).toContain("  return Reconciler;\n");
  expect(js).toContain("  return { ...Reconciler };\n");
  // As a value, given the namespace it's called on.
  expect(js).toContain("  return (a, b) => a.updateContainer(b);\n");
  expect(ts).toContain('import type * as Reconciler from "./reconciler.js";\n');
  expect(ts).toContain(
    "export function through(renderer: typeof Reconciler): number;\n",
  );
  expect(readFileSync(join(dir, "reconciler.d.ts"), "utf8")).toContain(
    "/** Makes a root of `tag`. */\nexport function createContainer(tag: number): number;\n\nexport function updateContainer(n: number): number;\n",
  );
  const lib = await import(join(dir, "lib.js"));
  const namespace = await import(join(dir, "reconciler.js"));
  expect(lib.direct()).toBe(7);
  // A fresh instance of the module, as Jest's mock gives React's renderer.
  expect(
    lib.through({
      createContainer: (n: number) => n + 100,
      updateContainer: (n: number) => n,
    }),
  ).toBe(105);
  expect(lib.through(namespace)).toBe(11);
  expect(lib.namespace()).toBe(namespace);
  expect(lib.method()({ updateContainer: (n: number) => n + 1 }, 2)).toBe(3);
  expect(Object.keys(lib.config())).toEqual([
    "createContainer",
    "me",
    "roots",
    "updateContainer",
  ]);
  // Its own module's value, the module importing itself.
  expect(module).toContain('import * as Reconciler from "./reconciler.js";\n');
  expect(namespace.me()).toBe(namespace);
  expect(namespace.roots()).toBe(2);
});

test("what a namespace is refused says what to write", () => {
  const head = "#[cfg_attr(rust_js, rust_js::namespace)]\npub struct N;\n";
  const marked = "#[cfg_attr(rust_js, rust_js::namespace)]\nimpl N {\n";
  const get = "    pub fn get(&self) -> u32 {\n        1\n    }\n";
  const cases: [string, string, string][] = [
    [
      "namespace-fields",
      "#[cfg_attr(rust_js, rust_js::namespace)]\npub struct N {\n    pub n: u32,\n}\n",
      "is a unit struct, `pub struct ReactFiberReconciler;`",
    ],
    [
      "namespace-unmarked-impl",
      `${head}impl N {\n${get}}\n`,
      "struct's impl is marked `#[rust_js::namespace]` too",
    ],
    [
      "namespace-unmarked-struct",
      `pub struct N;\n${marked}${get}}\n`,
      "impl is of a struct marked `#[rust_js::namespace]`",
    ],
    [
      "namespace-by-value",
      `${head}${marked}    pub fn take(self) -> u32 {\n        1\n    }\n}\n`,
      "has methods of `&self` alone",
    ],
    [
      "namespace-static",
      `${head}${marked}    pub fn make() -> u32 {\n        1\n    }\n}\n`,
      "has methods of `&self` alone",
    ],
    [
      "namespace-self-value",
      `${head}${marked}    pub fn me(&self) -> &Self {\n        self\n    }\n}\n`,
      "reads `self` only to call its functions",
    ],
    [
      "namespace-elsewhere",
      `pub mod other {\n    ${head}}\n#[cfg_attr(rust_js, rust_js::namespace)]\nimpl other::N {\n${get}}\n`,
      "impl is in its struct's module",
    ],
    [
      "namespace-and-object",
      "#[cfg_attr(rust_js, rust_js::namespace, rust_js::object)]\npub struct N;\n",
      "isn't a `#[rust_js::class]` or `#[rust_js::object]` too",
    ],
  ];
  for (const [name, rust, message] of cases) {
    const dir = fixture(name);
    writeFileSync(join(dir, "lib.rs"), rust);
    const ran = runSync([compiler, ...flags(dir)], root, 60_000);
    expect(ran.code).not.toBe(0);
    expect(ran.stderr).toContain(message);
  }
});

// React's split: the reconciler a library, React DOM calling its functions
// by name, the noop renderer through the value a binding gives, its
// `createReconciler(hostConfig)`'s, Jest's instance of the module.
test("a library's namespace is called by name or through the value a binding gives", async () => {
  const dir = fixture("namespace-library");
  const js = ["--extern", `js=${join(target, "libjs.rmeta")}`, "-L", target];
  writeFileSync(join(dir, "dep.rs"), "pub mod reconciler;\n");
  writeFileSync(join(dir, "reconciler.rs"), reconciler);
  run([
    compiler,
    join(dir, "dep.rs"),
    "-o",
    join(dir, "dep", "lib.js"),
    "--library",
    "--manifest",
    join(dir, "dep", "lib.manifest.json"),
    "--",
    "--crate-name",
    "dep",
    `--emit=metadata=${join(dir, "libdep.rmeta")}`,
    ...js,
  ]);
  writeFileSync(
    join(dir, "make.js"),
    "export default (config) => ({ createContainer: (n) => n + config.base, updateContainer: (n) => n });\n",
  );
  writeFileSync(
    join(dir, "app.rs"),
    `use dep::reconciler::Reconciler;

pub struct Config {
    pub base: u32,
}

unsafe impl js::Defined for Config {}

#[cfg_attr(rust_js, rust_js::link_name = "../make.js#default")]
fn create_reconciler(_config: &Config) -> &'static Reconciler {
    unreachable!()
}

pub fn dom() -> u32 {
    Reconciler.create_container(3)
}

pub fn namespace() -> &'static Reconciler {
    &Reconciler
}

pub fn noop() -> u32 {
    let noop_renderer = create_reconciler(&Config { base: 100 });
    noop_renderer.create_container(4) + noop_renderer.update_container(1)
}
`,
  );
  run([
    compiler,
    "--declarations",
    join(dir, "app.rs"),
    "-o",
    join(dir, "app", "lib.js"),
    "--dependency",
    join(dir, "dep", "lib.manifest.json"),
    "--",
    "--crate-name",
    "app",
    "--extern",
    `dep=${join(dir, "libdep.rmeta")}`,
    "-L",
    dir,
    ...js,
  ]);
  const app = readFileSync(join(dir, "app", "lib.js"), "utf8");
  const lib = await import(join(dir, "app", "lib.js"));
  expect(app).toContain(
    'import { createContainer } from "../dep/reconciler.js";\n',
  );
  expect(app).toContain("  return createContainer(3);\n");
  expect(app).toContain(
    "  const noopRenderer = createReconciler({ base: 100 });\n",
  );
  expect(app).toContain(
    "  return (noopRenderer.createContainer(4) + noopRenderer.updateContainer(1)) >>> 0;\n",
  );
  expect(app).toContain(
    'import * as Reconciler from "../dep/reconciler.js";\n',
  );
  expect([lib.dom(), lib.noop(), lib.namespace().updateContainer(2)]).toEqual([
    7, 105, 4,
  ]);
});
