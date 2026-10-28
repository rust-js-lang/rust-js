// What a crate's declarations (ADR 0196) say of a library's types: each one
// the library's own declarations export, imported from its file, as its
// JS imports the library's functions (ADR 0210, amended), so a TypeScript
// caller is held to it.

import { beforeAll, expect, test } from "bun:test";
import { existsSync, mkdirSync, readFileSync, writeFileSync } from "node:fs";
import { join } from "node:path";
import { buildReact, compiler, fixture, root, run, target } from "./support";

beforeAll(buildReact, 600_000);

const js = ["--extern", `js=${join(target, "libjs.rmeta")}`, "-L", target];

/** `dep`'s modules, a library, and `app`, a crate that uses it, compiled
 * in `dir`, `app` with its declarations, `dep` with its own unless
 * `undeclared`. */
function compiled(dir: string, dep: Record<string, string>, app: string, undeclared = false) {
  mkdirSync(join(dir, "dep"), { recursive: true });
  for (const [file, source] of Object.entries(dep)) {
    writeFileSync(join(dir, file), source);
  }
  run([compiler, ...(undeclared ? [] : ["--declarations"]), join(dir, "dep.rs"), "-o", join(dir, "dep", "lib.js"), "--library", "--manifest", join(dir, "dep", "lib.manifest.json"),
    "--", "--crate-name", "dep", `--emit=metadata=${join(dir, "libdep.rmeta")}`, ...js]);
  writeFileSync(join(dir, "app.rs"), app);
  run([compiler, "--declarations", join(dir, "app.rs"), "-o", join(dir, "app", "lib.js"), "--dependency", join(dir, "dep", "lib.manifest.json"),
    "--", "--crate-name", "app", "--extern", `dep=${join(dir, "libdep.rmeta")}`, "-L", dir, ...js]);
}

/** TypeScript's errors in `file` of `dir`, strict, its declarations checked too. */
function typeChecked(dir: string, file: string): string[] {
  const config = join(dir, "tsconfig.json");
  writeFileSync(config, JSON.stringify({
    compilerOptions: { module: "esnext", moduleResolution: "bundler", strict: true, noEmit: true, skipLibCheck: false },
    files: [file],
  }));
  const checked = Bun.spawnSync([process.execPath, join(root, "node_modules/typescript/bin/tsc"), "-p", config], { cwd: dir });
  return checked.stdout.toString().split("\n").filter((line) => line.includes("error TS"));
}

const contexts = {
  "dep.rs": "pub mod context;\n",
  "context.rs": `use std::cell::Cell;

pub struct Context<T: 'static> {
    pub value: Cell<T>,
    pub count: Cell<f64>,
}

pub fn count(context: &Context<f64>) -> f64 {
    context.count.get()
}
`,
};

const reader = `use dep::context::Context;

pub fn read(context: &Context<f64>) -> f64 {
    context.value.get() + dep::context::count(context)
}
`;

// React's reconciler reads `CacheContext`, a `ReactContext<Cache>` of the
// react crate's, whose declaration was `any`: a library's type, which
// its own declarations export, is imported from its module's file.
test("a library's type is imported from the file its declarations are in", () => {
  const dir = fixture("declarations-library-types");
  compiled(dir, contexts, reader);
  const declarations = readFileSync(join(dir, "app", "lib.d.ts"), "utf8");
  expect(declarations).toContain('import type { Context } from "../dep/context.js";\n');
  expect(declarations).toContain("export function read(context: Context<number>): number;");
  writeFileSync(join(dir, "caller.ts"), `import { read } from "./app/lib.js";

read({ value: 1, count: 2 });
read({ value: "one", count: 2 });
`);
  const errors = typeChecked(dir, "caller.ts");
  expect(errors).toHaveLength(1);
  expect(errors[0]).toContain("caller.ts(4,");
});

// A library written without its declarations has no `.d.ts` to import a
// type from: its type is still `any`.
test("a library without declarations has its types as any", () => {
  const dir = fixture("declarations-library-undeclared");
  compiled(dir, contexts, reader, true);
  const declarations = readFileSync(join(dir, "app", "lib.d.ts"), "utf8");
  expect(declarations).not.toContain("import");
  expect(declarations).toContain("export function read(context: any): number;");
});

// A library's module's default, a class `js::export_default!` makes it
// (ADR 0761, amended), is imported as its default, as another module of
// the crate's is.
test("a library's default class is imported as its default", () => {
  const dir = fixture("declarations-library-default");
  compiled(dir, {
    "dep.rs": "pub mod complex;\n",
    "complex.rs": `#[cfg_attr(rust_js, rust_js::class)]
pub struct Complex {
    pub re: f64,
    pub im: f64,
}

#[cfg_attr(rust_js, rust_js::class)]
impl Complex {
    pub fn new(re: f64, im: f64) -> Self {
        Complex { re, im }
    }

    pub fn radius(&self) -> f64 {
        (self.re * self.re + self.im * self.im).sqrt()
    }
}

js::export_default!(Complex);
`,
  }, `use dep::complex::Complex;

pub fn radius(complex: &Complex) -> f64 {
    complex.radius()
}
`);
  const declarations = readFileSync(join(dir, "app", "lib.d.ts"), "utf8");
  expect(declarations).toContain('import type Complex from "../dep/complex.js";\n');
  expect(declarations).toContain("export function radius(complex: Complex): number;");
});

// A library's module of types alone, React DOM's `PluginModuleType`, is its
// `.d.ts` alone (the user's decision, ADR 1005): a consumer imports its
// types from where its JS would be, which TypeScript finds the `.d.ts` of.
test("a library's module of types alone gives its types to its consumer's declarations", () => {
  const dir = fixture("declarations-library-types-only");
  compiled(dir, {
    "dep.rs": "pub mod plugin_types;\n\npub fn flags() -> plugin_types::Flags {\n    1\n}\n",
    "plugin_types.rs": "pub type PluginName = String;\npub type Flags = i32;\n\npub struct DispatchEvent {\n    pub name: PluginName,\n}\n",
  }, `use dep::plugin_types::{DispatchEvent, PluginName};

pub fn named(event: &DispatchEvent) -> PluginName {
    event.name.clone() + "!"
}
`);
  expect([existsSync(join(dir, "dep", "plugin_types.js")), readFileSync(join(dir, "dep", "plugin_types.d.ts"), "utf8")])
    .toEqual([false, expect.stringContaining("export type PluginName = string;")]);
  const declarations = readFileSync(join(dir, "app", "lib.d.ts"), "utf8");
  expect(declarations).toContain('import type { DispatchEvent } from "../dep/plugin_types.js";\n');
  writeFileSync(join(dir, "caller.ts"), `import { named } from "./app/lib.js";

named({ name: "one" });
named({ name: 1 });
`);
  const errors = typeChecked(dir, "caller.ts");
  expect(errors).toHaveLength(1);
  expect(errors[0]).toContain("caller.ts(4,");
});
