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

// ReactFlightReplyClient's `ReactServerValue`, a union of JS types upstream
// exports, which processReply takes and reads by `js::classify`: an alias of
// `js::Unknown` whose `#[rust_js::types]` writes a type of its own, not a
// name, is declared as written, by its name, as its own module's and a
// consumer's declarations name it, imported from its file (rd538, ADR 0811
// amended); `readonly [key: string]: T` is Flow's `{+[key: string]: T}`.
test("an alias whose rust_js::types writes a union is declared as written and named where used", () => {
  const dir = fixture("declarations-library-written-alias");
  compiled(dir, {
    "dep.rs": "pub mod reply;\n",
    "reply.rs": `use js::Unknown;

pub type ServerReference<T> = T;

/// What a server reply holds.
#[cfg_attr(
    rust_js,
    rust_js::types = "ServerReference<() => void> | string | boolean | number | null | undefined | ReactServerValue[] | Map<ReactServerValue, ReactServerValue> | { readonly [key: string]: ReactServerValue } | Promise<ReactServerValue>"
)]
pub type ReactServerValue = Unknown;

pub fn process(root: Option<&'static ReactServerValue>) -> bool {
    root.is_some()
}
`,
  }, `use dep::reply::ReactServerValue;

pub fn encode_reply(value: &'static ReactServerValue) -> bool {
    dep::reply::process(Some(value))
}
`);
  const own = readFileSync(join(dir, "dep", "reply.d.ts"), "utf8");
  expect(own).toContain("/** What a server reply holds. */\nexport type ReactServerValue = ServerReference<() => void> | string | boolean | number | null | undefined | ReactServerValue[] | Map<ReactServerValue, ReactServerValue> | {\n    readonly [key: string]: ReactServerValue;\n} | Promise<ReactServerValue>;");
  expect(own).toContain("export function process(root?: ReactServerValue | null | undefined): boolean;");
  const declarations = readFileSync(join(dir, "app", "lib.d.ts"), "utf8");
  expect(declarations).toContain('import type { ReactServerValue } from "../dep/reply.js";\n');
  expect(declarations).toContain("export function encodeReply(value: ReactServerValue): boolean;");
  writeFileSync(join(dir, "caller.ts"), `import { encodeReply } from "./app/lib.js";

encodeReply(new Map([["a", [1, "b", { c: true }]]]));
encodeReply(Symbol());
`);
  const errors = typeChecked(dir, "caller.ts");
  expect(errors).toHaveLength(1);
  expect(errors[0]).toContain("caller.ts(4,");
});

// React's server entries: upstream's ReactFlightDOMServerNode re-exports
// ReactFlightActionServer's `decodeAction`, generic over the bundler's
// manifest, whose port takes webpack's. A function that only passes its
// parameters on, in order, each as it is, to a plain function at one
// instantiation is that function, re-exported by its own name, `export {
// decode as decodeAction }`, and declared at that instantiation, `manifest:
// ServerManifest`, not `M` (rd575, ADR 1064). One that does anything more,
// or whose call is JS of its own, stays a function.
test("a function that only passes its parameters on to one instantiation is its re-export", async () => {
  const dir = fixture("declarations-forwarders");
  compiled(dir, {
    "dep.rs": "pub mod action;\npub mod server;\n",
    "action.rs": `pub fn decode<M: ?Sized>(body: u32, _manifest: &M) -> u32 {
    body + 1
}

pub fn pair(a: u32, b: u32) -> u32 {
    a * 10 + b
}

pub fn maybe(x: Option<u32>) -> bool {
    x.is_some()
}

pub fn ping(n: u32) {
    println!("ping {n}");
}

pub fn hold<T>(value: T) -> u32 {
    drop(value);
    1
}

pub fn shown<T: std::fmt::Display>(value: T) -> String {
    format!("<{value}>")
}

pub fn bump<T>(value: &mut T) -> &mut T {
    value
}

pub fn kept(n: u32) -> u32 {
    n
}

pub fn some<T: Copy>(x: Option<T>) -> bool {
    x.is_some()
}

pub fn hold_plain<T>(value: T) -> u32 {
    drop(value);
    1
}

pub struct Noisy;

impl Drop for Noisy {
    fn drop(&mut self) {
        println!("noisy");
    }
}

pub fn holds_noisy() -> u32 {
    hold_plain(Noisy)
}

pub fn given(_first: Option<&js::Unknown>) -> Option<&'static js::Unknown> {
    js::get(js::arguments(), "length")
}
`,
    "server.rs": `pub struct Manifest {
    pub id: u32,
}

/// Decodes with the server's manifest.
pub fn decode(body: u32, manifest: &Manifest) -> u32 {
    crate::action::decode::<Manifest>(body, manifest)
}
`,
  }, `use dep::action;

pub struct ServerManifest {
    pub id: u32,
}

pub struct Guard(pub u32);

impl Drop for Guard {
    fn drop(&mut self) {
        println!("drop {}", self.0);
    }
}

pub struct Point {
    pub x: u32,
}

pub fn decode_action(body: u32, server_manifest: &ServerManifest) -> u32 {
    action::decode::<ServerManifest>(body, server_manifest)
}

pub fn decode_form_state(body: u32, server_manifest: &ServerManifest) -> u32 {
    action::decode(body, server_manifest)
}

pub fn ping(n: u32) {
    action::ping(n)
}

pub fn server_decode(body: u32, manifest: &dep::server::Manifest) -> u32 {
    dep::server::decode(body, manifest)
}

pub fn swapped(a: u32, b: u32) -> u32 {
    action::pair(b, a)
}

pub fn wraps(x: u32) -> bool {
    action::maybe(Some(x))
}

pub fn logs(n: u32) -> u32 {
    println!("logs");
    action::kept(n)
}

pub fn holds(guard: Guard) -> u32 {
    action::hold(guard)
}

pub fn holds_plain(n: u32) -> u32 {
    action::hold_plain(n)
}

pub fn nested(x: Option<Option<u32>>) -> bool {
    action::some(x)
}

pub fn counted(first: Option<&js::Unknown>) -> Option<&'static js::Unknown> {
    action::given(first)
}

pub fn shows(n: u32) -> String {
    action::shown(n)
}

pub fn bumped(point: &mut Point) -> &mut Point {
    action::bump(point)
}

pub fn discards(n: u32) {
    action::kept(n);
}

pub fn generic<M: ?Sized>(body: u32, manifest: &M) -> u32 {
    action::decode(body, manifest)
}
`);
  const server = readFileSync(join(dir, "dep", "server.js"), "utf8");
  expect(server).toContain('/** Decodes with the server\'s manifest. */\nexport { decode } from "./action.js";\n');
  expect(server).not.toContain("function");
  expect(readFileSync(join(dir, "dep", "server.d.ts"), "utf8")).toContain("export function decode(body: number, manifest: Manifest): number;");
  const app = readFileSync(join(dir, "app", "lib.js"), "utf8");
  // A run of them, from one module, one statement; a library's forwarder
  // from its own file, which re-exports it.
  // The library's module's import where its `use` is, first.
  expect(app).toContain('import * as Action from "../dep/action.js";\nexport { decode as decodeAction');
  expect(app).toContain(`export { decode as decodeAction, decode as decodeFormState, ping } from "../dep/action.js";
export { decode as serverDecode } from "../dep/server.js";
`);
  for (const kept of ["swapped(a, b)", "wraps(x)", "logs(n)", "holds(guard)", "holdsPlain(n)", "nested(x)", "counted(first)", "shows(n)", "bumped(point)", "discards(n)", "generic(body, manifest)"]) {
    expect(app).toContain(`export function ${kept} {`);
  }
  expect(app).not.toMatch(/function (decodeAction|decodeFormState|ping|serverDecode)\b/);
  // What only a re-exported function read isn't imported.
  expect(app).not.toContain("import * as Server");
  const declarations = readFileSync(join(dir, "app", "lib.d.ts"), "utf8");
  expect(declarations).toContain("export function decodeAction(body: number, serverManifest: ServerManifest): number;");
  expect(declarations).toContain("export function ping(n: number): void;");
  expect(declarations).toContain('import type { Manifest } from "../dep/server.js";\n');
  expect(declarations).toContain("export function serverDecode(body: number, manifest: Manifest): number;");
  expect(declarations).not.toContain("export {");
  writeFileSync(join(dir, "caller.ts"), `import { decodeAction, serverDecode } from "./app/lib.js";

decodeAction(1, { id: 2 });
serverDecode(1, { id: 2 });
decodeAction(1, "manifest");
`);
  const errors = typeChecked(dir, "caller.ts");
  expect(errors).toHaveLength(1);
  expect(errors[0]).toContain("caller.ts(5,");
  const module = await import(join(dir, "app", "lib.js"));
  const action = await import(join(dir, "dep", "action.js"));
  expect([module.decodeAction(1, { id: 2 }), module.decodeFormState(2, { id: 2 }), module.serverDecode(5, { id: 1 })]).toEqual([2, 3, 6]);
  expect([module.decodeAction, module.decodeFormState, module.serverDecode]).toEqual([action.decode, action.decode, action.decode]);
  expect(module.ping).toBe(action.ping);
  expect([module.swapped(1, 2), module.wraps(3), module.holds({ 0: 4 }), module.shows(5), module.generic(6, {})]).toEqual([21, true, 1, "<5>", 7]);
  // What a JS caller gives past their parameters, none of theirs.
  expect([module.holdsPlain(4, 5), module.counted(), module.nested({ value: undefined })]).toEqual([1, 1, true]);
});
