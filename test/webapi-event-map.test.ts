// A binding trait's projection to TypeScript (#1817): webapi's
// `<Target as Listen<Name>>::Event` is `@rust-js/webapi`'s
// `EventMapOf<Target>[Name]`, the map `webapi/index.d.ts` declares of the
// crate's `Listen` impls, and a bound `Target: Listen<Name>` constrains
// `Name extends keyof EventMapOf<Target>`. The vercel UI kernel's `on`,
// upstream's `on<Type extends keyof HTMLElementEventMap>(target, type,
// listener: (event: HTMLElementEventMap[Type]) => void, options?)`
// (vercel-labs/ui@220c1a0, packages/core/src/foundation/kernel/events.ts),
// is declared per target, and Codex's consumer of it type-checks
// (rust-js-lang/vercel-ui-sdk-in-rust@da281b6, tests/types/kernel-events.ts).

import { beforeAll, expect, test } from "bun:test";
import { readFileSync, writeFileSync } from "node:fs";
import { join } from "node:path";

import { typescriptEventMaps } from "../webapi/coverage";
import { buildReact, compiler, fixture, root, run, target } from "./support";
import { underLoad } from "../scripts/timing";

beforeAll(buildReact, 600_000);

const withWeb = ["--", "--extern", `webapi=${join(target, "libwebapi.rmeta")}`, "--extern", `js=${join(target, "libjs.rmeta")}`, "-L", target];

/** TypeScript's errors of `file` in `dir`, strict, the DOM's lib in. */
function typeChecked(dir: string, file: string): string[] {
  const config = join(dir, `tsconfig-${file}.json`);
  writeFileSync(config, JSON.stringify({
    compilerOptions: { module: "esnext", moduleResolution: "bundler", target: "es2022", lib: ["es2022", "dom"], strict: true, noEmit: true, skipLibCheck: false, exactOptionalPropertyTypes: true, types: [] },
    files: [file],
  }));
  const checked = Bun.spawnSync([process.execPath, join(root, "node_modules/typescript/bin/tsc"), "-p", config], { cwd: dir });
  return checked.stdout.toString().split("\n").filter((line) => line.includes("error TS"));
}

/** The crate's `Listen` catalog, of its generated source: each target's
 * events, by name, each the event's interface. */
function catalog(): Map<string, Map<string, string>> {
  const lib = readFileSync(join(root, "webapi/src/lib.rs"), "utf8");
  const names = new Map<string, string>();
  const events = lib.slice(lib.indexOf("pub mod events {"));
  for (const m of events.slice(0, events.indexOf("\n}\n")).matchAll(/rust_js::name = "([^"]+)"\)\]\n(?:\s*#\[[^\n]*\]\n)*\s*pub struct (\w+);/g)) names.set(m[2], m[1]);
  const listens = new Map<string, Map<string, string>>();
  for (const m of lib.matchAll(/^impl Listen<events::(\w+)> for (\w+) \{ type Event = (\w+); \}$/gm)) {
    if (!listens.has(m[2])) listens.set(m[2], new Map());
    listens.get(m[2])!.set(names.get(m[1])!, m[3]);
  }
  return listens;
}

const program = `use webapi::{AddEventListenerOptions, EventTargetExt, Listen};

/// Adds a listener and returns what removes it.
pub fn on<'a, Target, Name>(
    target: &'a Target,
    name: Name,
    listener: &'static dyn Fn(&'static <Target as Listen<Name>>::Event),
    options: Option<&'a AddEventListenerOptions<'a>>,
) -> impl Fn() + 'a
where
    Target: EventTargetExt + Listen<Name>,
    Name: Copy + 'a,
{
    target.add_event_listener(name, listener, options.map(|options| options as &dyn webapi::IntoAddEventListenerOptionsOrBool));
    move || target.remove_event_listener(name, listener, options.map(|options| options as &dyn webapi::IntoEventListenerOptionsOrBool))
}

/// Of a bound alone, its projection named nowhere.
pub fn listened<Target, Name>(target: &Target, name: Name, listener: &'static dyn Fn(&'static webapi::Event))
where
    Target: EventTargetExt + Listen<Name>,
{
    target.add_event_listener_named("listened", listener, None);
    let _ = name;
}

pub async fn battery(navigator: &'static webapi::Navigator) -> &'static webapi::BatteryManager {
    navigator.get_battery().await
}
`;

test("a binding trait's projection is its type function's, and a bound constrains the key", () => {
  const dir = fixture("webapi-event-map");
  writeFileSync(join(dir, "lib.rs"), program);
  run([compiler, "--declarations", join(dir, "lib.rs"), "-o", join(dir, "lib.js"), ...withWeb]);
  const declarations = readFileSync(join(dir, "lib.d.ts"), "utf8");
  expect(declarations).toContain('import type { EventMapOf } from "@rust-js/webapi";');
  expect(declarations).toContain(
    "export function on<Target, Name extends keyof EventMapOf<Target>>(target: Target, name: Name, listener: (event: EventMapOf<Target>[Name]) => void, options?: AddEventListenerOptions | null | undefined): () => void;",
  );
  expect(declarations).toContain("export function listened<Target, Name extends keyof EventMapOf<Target>>(target: Target, name: Name, listener: (event: Event) => void): void;");
  // Codex's consumer, as written but its import: a click's `clientX`, a
  // keydown's `key`, a wrong capture and an event no target has refused.
  writeFileSync(join(dir, "kernel-events.ts"), `import {on} from "./lib.js";

const button = document.createElement("button");
on(button, "click", event => {
  event.preventDefault();
  event.clientX;
  // @ts-expect-error Native click events have no arbitrary property.
  event.nonexistentEventProperty;
}, {capture: true, once: true});

on(document, "keydown", event => { event.key; }, {capture: false});

// @ts-expect-error Capture must be a boolean.
on(button, "click", () => {}, {capture: "wrong"});

// @ts-expect-error The event name must belong to the native event map.
on(button, "not-a-native-event", () => {});
`);
  expect(typeChecked(dir, "kernel-events.ts")).toEqual([]);
  // A listener narrower than its event, or a name of no target's, refused;
  // of a bound alone too.
  writeFileSync(join(dir, "wrong.ts"), `import { listened, on } from "./lib.js";
const button = document.createElement("button");
on(button, "click", (event: KeyboardEvent) => event.key);
listened(button, "not-an-event", () => {});
`);
  expect(typeChecked(dir, "wrong.ts")).toEqual([expect.stringContaining("error TS2345"), expect.stringContaining("error TS2345")]);
});

// Each target's map is the catalog's, exactly: its names, each its event,
// or, where TypeScript can't tell the target from a parent, an
// `XMLHttpRequestUpload` from its `XMLHttpRequestEventTarget`, the
// parent's, the wider; which, tsc says, of lib.dom's own `extends`.
test("every target's EventMapOf is its catalog's map, by tsc", async () => {
  const dir = fixture("webapi-event-map-targets");
  const listens = catalog();
  const { extendsOf } = await typescriptEventMaps();
  const ancestors = (name: string, seen = new Set<string>()): string[] =>
    (extendsOf.get(name) ?? []).flatMap((parent) => (seen.has(parent) ? [] : (seen.add(parent), [...ancestors(parent, seen), parent])));
  const declares = new Set(["BatteryManager", "HTMLSelectedContentElement", "SpeechRecognition"]);
  const map = (t: string) => `{ ${[...listens.get(t)!].map(([name, event]) => `${JSON.stringify(name)}: ${event}`).join("; ")} }`;
  let text = `import type { EventMapOf } from "@rust-js/webapi";
type Is<T, X> = [T] extends [X] ? ([X] extends [T] ? true : false) : false;
type Same<A, B> = [A] extends [B] ? ([B] extends [A] ? true : false) : false;
type Exact<M, E> = Same<keyof M, keyof E> extends true ? ({ [K in keyof E]: K extends keyof M ? Same<M[K], E[K]> : false }[keyof E] extends true ? true : false) : false;
type Expect<T extends true> = T;
`;
  for (const t of listens.keys()) {
    if (declares.has(t)) continue;
    // Its parents of a map, shallowest first, each if tsc can't tell it from them.
    const parents = ancestors(t).filter((parent) => listens.has(parent));
    const expected = parents.reduceRight((rest, parent) => `true extends Is<${t}, ${parent}> ? ${map(parent)} : ${rest}`, map(t));
    text += `export type _${t} = Expect<Exact<EventMapOf<${t}>, ${expected}>>;\n`;
  }
  writeFileSync(join(dir, "targets.ts"), text);
  expect(typeChecked(dir, "targets.ts")).toEqual([]);
  // And a wrong one isn't, so the check checks.
  writeFileSync(join(dir, "wrong.ts"), `${text}export type _wrong = Expect<Exact<EventMapOf<HTMLButtonElement>, { click: KeyboardEvent }>>;\n`);
  expect(typeChecked(dir, "wrong.ts")).toEqual([expect.stringContaining("error TS2344")]);
}, underLoad(120_000));

// A type TypeScript's DOM lacks is an empty global interface of
// index.d.ts's, filled by a consumer's own declaration of it
// (`rust_js::declares`): then its map; and an undeclared one matches only
// what's empty, never another type.
test("a type TypeScript lacks gets its map where a crate declares it, and matches nothing where none does", () => {
  const dir = fixture("webapi-event-map-declared");
  writeFileSync(join(dir, "lib.rs"), program);
  run([compiler, "--declarations", join(dir, "lib.rs"), "-o", join(dir, "lib.js"), ...withWeb]);
  expect(readFileSync(join(dir, "lib.d.ts"), "utf8")).toContain("interface BatteryManager extends EventTarget {");
  writeFileSync(join(dir, "declared.ts"), `import type { EventMapOf } from "@rust-js/webapi";
import { battery } from "./lib.js";
type Same<A, B> = [A] extends [B] ? ([B] extends [A] ? true : false) : false;
type Expect<T extends true> = T;
const manager = await battery(navigator);
export type Level = Expect<Same<EventMapOf<typeof manager>["levelchange"], Event>>;
export type Names = Expect<Same<keyof EventMapOf<BatteryManager>, "chargingchange" | "chargingtimechange" | "dischargingtimechange" | "levelchange">>;
// Undeclared: none of another type's.
export type Other = Expect<Same<keyof EventMapOf<{ unrelated: true }>, never>>;
`);
  expect(typeChecked(dir, "declared.ts")).toEqual([]);
});

// What @rust-js/webapi ships for it, pinned: a map grows only as the
// catalog does (44 KB of 242 targets, 34 maps of their own).
test("webapi's event map is within its size", () => {
  expect(readFileSync(join(root, "webapi/index.d.ts")).length).toBeLessThanOrEqual(44 * 1024);
});
