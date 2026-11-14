// A type webapi binds that TypeScript's DOM doesn't declare, the Battery
// Status API's `BatteryManager`, a spec read whole of its IDL (#1790), or
// DOM's `NodeRange`: the generator says so,
// `#[rust_js::declares]`, of each bound type `@types/web` lacks, and a
// crate's declarations that name one declare it, `declare global {
// interface BatteryManager extends EventTarget { .. } }`, of the binding,
// so a consumer's `.d.ts` type-checks under strict tsc.

import { beforeAll, expect, test } from "bun:test";
import { readFileSync, writeFileSync } from "node:fs";
import { join } from "node:path";

import { typescriptNames } from "../webapi/coverage";
import { buildReact, compiler, fixture, root, run, target } from "./support";

beforeAll(buildReact, 600_000);

const withWeb = ["--", "--extern", `webapi=${join(target, "libwebapi.rmeta")}`, "--extern", `js=${join(target, "libjs.rmeta")}`, "-L", target];

/** TypeScript's errors in `file` of `dir`, strict, the DOM's lib in. */
function typeChecked(dir: string, file: string): string[] {
  const config = join(dir, "tsconfig.json");
  writeFileSync(config, JSON.stringify({
    compilerOptions: { module: "esnext", moduleResolution: "bundler", target: "es2022", lib: ["es2022", "dom"], strict: true, noEmit: true, skipLibCheck: false, exactOptionalPropertyTypes: true, types: [] },
    files: [file],
  }));
  const checked = Bun.spawnSync([process.execPath, join(root, "node_modules/typescript/bin/tsc"), "-p", config], { cwd: dir });
  return checked.stdout.toString().split("\n").filter((line) => line.includes("error TS"));
}

test("navigator.getBattery() is bound, and its BatteryManager declared where TypeScript lacks it", async () => {
  const dir = fixture("webapi-battery");
  writeFileSync(join(dir, "lib.rs"), `pub async fn level(navigator: &'static webapi::Navigator) -> f64 {
    navigator.get_battery().await.level()
}

pub async fn manager(navigator: &'static webapi::Navigator) -> &'static webapi::BatteryManager {
    navigator.get_battery().await
}
`);
  run([compiler, "--declarations", join(dir, "lib.rs"), "-o", join(dir, "lib.js"), ...withWeb]);
  const declarations = readFileSync(join(dir, "lib.d.ts"), "utf8");
  expect(declarations).toContain("declare global {\n    interface BatteryManager extends EventTarget {\n        readonly charging: boolean;");
  expect(declarations).toContain("        readonly level: number;\n");
  expect(declarations).toContain("        onlevelchange: ((event: Event) => void) | null;\n");
  expect(declarations).toContain("export function manager(navigator: Navigator): Promise<BatteryManager>;");
  writeFileSync(join(dir, "use.ts"), `import { level, manager } from "./lib.js";
export const battery: BatteryManager = await manager(navigator);
export const charging: boolean = battery.charging;
export const percent: number = await level(navigator);
battery.onlevelchange = () => {};
`);
  expect(typeChecked(dir, "use.ts")).toEqual([]);
  writeFileSync(join(dir, "wrong.ts"), `import { manager } from "./lib.js";
const battery = await manager(navigator);
export const level: string = battery.level;
battery.charging = true;
`);
  expect(typeChecked(dir, "wrong.ts")).toEqual([expect.stringContaining("error TS2322"), expect.stringContaining("error TS2540")]);
  const lib = await import(join(dir, "lib.js"));
  const battery = { charging: true, level: 0.5 };
  const navigator = { getBattery: () => Promise.resolve(battery) };
  expect([await lib.level(navigator), await lib.manager(navigator)]).toEqual([0.5, battery]);
});

test("an HTMLSelectedContentElement a crate names is declared where TypeScript lacks it", () => {
  const dir = fixture("webapi-selected-content");
  writeFileSync(join(dir, "lib.rs"), `pub fn tag(element: &webapi::HTMLSelectedContentElement) -> String {
    element.tag_name()
}
`);
  run([compiler, "--declarations", join(dir, "lib.rs"), "-o", join(dir, "lib.js"), ...withWeb]);
  const declarations = readFileSync(join(dir, "lib.d.ts"), "utf8");
  expect(declarations).toContain("declare global {\n    interface HTMLSelectedContentElement extends HTMLElement {");
  expect(declarations).toContain("export function tag(element: HTMLSelectedContentElement): string;");
  writeFileSync(join(dir, "use.ts"), `import { tag } from "./lib.js";
declare const selected: HTMLSelectedContentElement;
export const name: string = tag(selected);
`);
  expect(typeChecked(dir, "use.ts")).toEqual([]);
});

// A webapi dictionary a public function takes is TypeScript's own of its
// name, `AddEventListenerOptions`, imported from nowhere as the DOM lib's,
// never `any` (#1817); one TypeScript's lib lacks, `BoxQuadOptions`, is
// declared of its fields, as a crate's own struct is.
test("a webapi dictionary a function takes is TypeScript's of its name, or declared of its fields", () => {
  const dir = fixture("webapi-dictionaries");
  writeFileSync(join(dir, "lib.rs"), `pub fn capturing(options: Option<&webapi::AddEventListenerOptions>) -> bool {
    options.and_then(|options| options.capture).unwrap_or(false)
}

pub fn boxed(options: &webapi::BoxQuadOptions) -> bool {
    options.r#box.is_some()
}
`);
  run([compiler, "--declarations", join(dir, "lib.rs"), "-o", join(dir, "lib.js"), ...withWeb]);
  const declarations = readFileSync(join(dir, "lib.d.ts"), "utf8");
  expect(declarations).toContain("export function capturing(options?: AddEventListenerOptions | null | undefined): boolean;");
  expect(declarations).toContain("export function boxed(options: BoxQuadOptions): boolean;");
  expect(declarations).toContain("declare global {\n    interface BoxQuadOptions {\n        box?: string | undefined;");
  writeFileSync(join(dir, "use.ts"), `import { boxed, capturing } from "./lib.js";
export const on: boolean = capturing({ capture: true, once: false, signal: new AbortController().signal });
export const off: boolean = capturing(undefined);
export const quad: boolean = boxed({ box: "border" });
`);
  expect(typeChecked(dir, "use.ts")).toEqual([]);
  writeFileSync(join(dir, "wrong.ts"), `import { capturing } from "./lib.js";
export const wrong: boolean = capturing({ capture: "yes" });
`);
  expect(typeChecked(dir, "wrong.ts")).toEqual([expect.stringContaining("error TS2322")]);
});

// DOM's WebIDL moved `startContainer` from `AbstractRange` to `NodeRange`,
// which `Range` and `StaticRange` inherit and TypeScript hasn't caught up
// with: bound, a `Range` derefs to it and reads it, and it's declared.
test("a Range reads its startContainer of NodeRange, which TypeScript lacks", async () => {
  const dir = fixture("webapi-node-range");
  writeFileSync(join(dir, "lib.rs"), `pub fn start(range: &webapi::Range) -> &'static webapi::Node {
    range.start_container()
}

pub fn of(range: &webapi::Range) -> &webapi::NodeRange {
    range
}

pub fn offset(range: &webapi::Range) -> f64 {
    range.start_offset().into()
}
`);
  run([compiler, "--declarations", join(dir, "lib.rs"), "-o", join(dir, "lib.js"), ...withWeb]);
  const declarations = readFileSync(join(dir, "lib.d.ts"), "utf8");
  expect(declarations).toContain("declare global {\n    interface NodeRange extends AbstractRange {\n        readonly startContainer: Node;");
  writeFileSync(join(dir, "use.ts"), `import { of, offset, start } from "./lib.js";
declare const range: Range;
export const node: Node = start(range);
export const container: Node = of(range).startContainer;
export const at: number = offset(range);
`);
  expect(typeChecked(dir, "use.ts")).toEqual([]);
  const lib = await import(join(dir, "lib.js"));
  const node = {};
  const range = { startContainer: node, startOffset: 3 };
  expect([lib.start(range), lib.of(range), lib.offset(range)]).toEqual([node, range, 3]);
});

// The generator says it of each type it binds that the pinned TypeScript's
// own DOM lib lacks, which a consumer's tsc reads, and of no other: its
// placement is that reader's, never a hand's. `@types/web` may be ahead.
test("rust_js::declares is on exactly the bound types TypeScript's DOM lacks", async () => {
  const crate = readFileSync(join(root, "webapi/src/lib.rs"), "utf8");
  const named = await typescriptNames();
  // Each struct's attributes, the lines of them right above it.
  const lines = crate.split("\n");
  const typed: { name: string; declares: boolean }[] = [];
  lines.forEach((line, at) => {
    if (!line.startsWith("pub struct ")) return;
    const attributes: string[] = [];
    for (let i = at - 1; i >= 0 && lines[i].startsWith("#["); i--) attributes.push(lines[i]);
    const name = attributes.map((a) => a.match(/^#\[cfg_attr\(rust_js, rust_js::types = "([A-Za-z_.][\w.]*)"\)\]$/)?.[1]).find(Boolean);
    if (name) typed.push({ name, declares: attributes.includes("#[cfg_attr(rust_js, rust_js::declares)]") });
  });
  expect(typed.length).toBeGreaterThan(500);
  const declared = typed.filter((t) => t.declares).map((t) => t.name).sort();
  const lacking = typed.filter((t) => !named.has(t.name.split(".")[0])).map((t) => t.name).sort();
  expect(declared).toEqual(lacking);
  // Which they are moves with each bump (`bun run definitions`): these two,
  // which TypeScript's lib lacks, the Battery Status API, Chromium's alone,
  // and DOM's `NodeRange`, new in WebIDL.
  expect(declared).toEqual(expect.arrayContaining(["BatteryManager", "NodeRange"]));
});

// A UI SDK selector's T extends Element is an IsA<Element> in Rust.
// Erasing that bound admits non-elements in a public TypeScript consumer.
test("exported WebAPI inheritance bounds constrain generic declarations", () => {
  const dir = fixture("webapi-inheritance-bounds");
  writeFileSync(join(dir, "lib.rs"), `use webapi::{Element, IsA as Extends};
pub fn selected<T: Extends<Element>>(value: &T) -> &T { value }
`);
  run([compiler, "--declarations", join(dir, "lib.rs"), "-o", join(dir, "lib.js"), ...withWeb]);
  const declarations = readFileSync(join(dir, "lib.d.ts"), "utf8");
  writeFileSync(join(dir, "valid.ts"), `import {selected} from "./lib.js";
const button: HTMLButtonElement = selected(document.createElement("button"));
const element: Element = selected<Element>(document.body);
void [button, element];
`);
  writeFileSync(join(dir, "invalid.ts"), `import {selected} from "./lib.js";
selected<string>("button");
selected<Text>(document.createTextNode("button"));
`);
  expect({declarations,valid:typeChecked(dir,"valid.ts"),invalid:typeChecked(dir,"invalid.ts")}).toEqual({
    declarations:expect.stringContaining("selected<T extends Element>"),
    valid:[],
    // A text node's is TS2740, its missing members, as tsc says of the
    // same declaration written by hand.
    invalid:[expect.stringContaining("error TS2344"),expect.stringContaining("error TS2740")],
  });
});
