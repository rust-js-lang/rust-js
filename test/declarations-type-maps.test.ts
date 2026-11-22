// A trait whose impls for named unit structs are one TypeScript object type
// (createDetails, ADR 1164): `#[rust_js::type_map(map = "ReasonVocabulary",
// keys = "ChangeReason", lookup = "EventOf")]` declares each impl's struct's
// name its key and its associated type its value, in the impls' order with
// the structs' doc comments, `keyof` it, and a lookup whose other strings
// take the `&'static str` impl's value, as upstream's details.ts
// (vercel-labs/ui@220c1a0) writes its vocabulary. A unit struct named as a
// string is that string's type, as its value is. Codex's contract:
// rust-js-lang/vercel-ui-sdk-in-rust@2680f613e,
// docs/create-details-contract.md.

import { beforeAll, expect, test } from "bun:test";
import { readFileSync, writeFileSync } from "node:fs";
import { join } from "node:path";
import { buildReact, compiler, fixture, root, run, target } from "./support";
import { es2022, type Setting, typeChecked } from "./type-checked";

beforeAll(buildReact, 600_000);

const withWeb = ["--", "--extern", `webapi=${join(target, "libwebapi.rmeta")}`, "--extern", `js=${join(target, "libjs.rmeta")}`, "-L", target];

// What this file's checks of its declarations are made under (`typeChecked`).
const checking: Setting = { ...es2022, lib: ["es2022", "dom"], types: [] };

const program = `use webapi::{Event, KeyboardEvent, MouseEvent};

/// Click, tap, Enter or Space on the part that opens.
#[cfg_attr(rust_js, rust_js::name = "trigger-press")]
#[derive(Clone, Copy, Debug)]
pub struct TriggerPress;

/// Escape.
#[cfg_attr(rust_js, rust_js::name = "escape-key")]
#[derive(Clone, Copy, Debug)]
pub struct EscapeKey;

/// The DOM event behind a press.
#[cfg_attr(rust_js, rust_js::untagged)]
pub enum PressEvent {
    Mouse(&'static MouseEvent),
    Keyboard(&'static KeyboardEvent),
}

/// Every reason a change can report, and the DOM event behind it.
#[cfg_attr(
    rust_js,
    rust_js::type_map(map = "ReasonVocabulary", keys = "ChangeReason", lookup = "EventOf")
)]
#[cfg_attr(rust_js, rust_js::types = "string")]
pub trait Reason: Copy {
    type Event;
}

impl Reason for TriggerPress {
    type Event = PressEvent;
}

impl Reason for EscapeKey {
    type Event = &'static KeyboardEvent;
}

impl Reason for &'static str {
    type Event = &'static Event;
}

/// The reason a press reports.
pub fn pressed() -> TriggerPress {
    TriggerPress
}

/// How the user caused it.
#[derive(Clone, Copy)]
pub enum ChangeSource {
    #[cfg_attr(rust_js, rust_js::name = "mouse")]
    Mouse,
    #[cfg_attr(rust_js, rust_js::name = "keyboard")]
    Keyboard,
}

/// What a commit's callback receives, one member per reason.
#[cfg_attr(rust_js, rust_js::distributive)]
pub struct CommitDetails<R: Reason = &'static str> {
    /// Why it changed.
    pub reason: R,
    /// The DOM event behind it.
    pub event: Option<R::Event>,
    /// How the user caused it.
    pub source: ChangeSource,
}

/// A press's commit.
pub fn committed(reason: TriggerPress) -> CommitDetails<TriggerPress> {
    CommitDetails { reason, event: None, source: ChangeSource::Mouse }
}
`;

test("a type map's impls are one object type, its keys and its lookup", () => {
  const dir = fixture("declarations-type-maps");
  writeFileSync(join(dir, "lib.rs"), program);
  run([compiler, "--declarations", join(dir, "lib.rs"), "-o", join(dir, "lib.js"), ...withWeb]);
  const declarations = readFileSync(join(dir, "lib.d.ts"), "utf8");
  expect(declarations).toContain('/** Click, tap, Enter or Space on the part that opens. */\nexport type TriggerPress = "trigger-press";');
  expect(declarations).toContain(`/** Every reason a change can report, and the DOM event behind it. */
export type ReasonVocabulary = {
    /** Click, tap, Enter or Space on the part that opens. */
    "trigger-press": PressEvent;
    /** Escape. */
    "escape-key": KeyboardEvent;
};`);
  expect(declarations).toContain("export type ChangeReason = keyof ReasonVocabulary;");
  expect(declarations).toContain("export type EventOf<Reason extends string> = Reason extends ChangeReason ? ReasonVocabulary[Reason] : Event;");
  expect(declarations).toContain("export function pressed(): TriggerPress;");
  // Of each reason, its event its own, the default any string (ADR 1164, C).
  expect(declarations).toContain(`/** What a commit's callback receives, one member per reason. */
export type CommitDetails<R extends string = string> = R extends unknown ? {
    /** Why it changed. */
    reason: R;
    /** The DOM event behind it. */
    event?: EventOf<R> | undefined;
    /** How the user caused it. */
    source: ChangeSource;
} : never;`);
  expect(readFileSync(join(dir, "lib.js"), "utf8")).toContain('export function pressed() {\n  return "trigger-press";\n}');
  writeFileSync(join(dir, "consumer.ts"), `import { committed, pressed, type ChangeReason, type CommitDetails, type EventOf, type ReasonVocabulary } from "./lib.js";
export function narrowed(details: CommitDetails<ChangeReason>): string {
  if (details.reason === "escape-key") return details.event?.key ?? "";
  return details.event instanceof MouseEvent ? \`\${details.event.clientX}\` : "";
}
export const any: CommitDetails = { reason: "anything", event: new Event("other"), source: "mouse" };
export const press: CommitDetails<"trigger-press"> = committed("trigger-press");
// @ts-expect-error An escape's event is a key's, never a mouse's.
export const wrongEvent: CommitDetails<"escape-key"> = { reason: "escape-key", event: new MouseEvent("click"), source: "mouse" };
export const reason: "trigger-press" = pressed();
export const reasons: ChangeReason[] = ["trigger-press", "escape-key"];
export const key: EventOf<"escape-key"> = new KeyboardEvent("keydown");
export const pressEvent: EventOf<"trigger-press"> = new MouseEvent("click");
export const other: EventOf<"anything"> = new Event("other");
export const vocabulary: ReasonVocabulary["escape-key"] = new KeyboardEvent("keydown");
// @ts-expect-error No such reason.
export const unknown: ChangeReason = "hover";
// @ts-expect-error An escape key's event is a key's.
export const wrong: EventOf<"escape-key"> = new MouseEvent("click");
`);
  expect(typeChecked(dir, "consumer.ts", checking)).toEqual([]);
});

// A `pub use` of a type map's trait re-exports its aliases, each by the
// name its attribute gives, from the module that declares them: the root's
// declarations name the map, its keys and its lookup, as the model's do
// (#2095); `as` renames the trait, not them.
test("a pub use of a type map's trait re-exports its map, keys and lookup", () => {
  const dir = fixture("declarations-type-map-reexport");
  writeFileSync(join(dir, "model.rs"), `/// Escape.
#[cfg_attr(rust_js, rust_js::name = "escape-key")]
#[derive(Clone, Copy)]
pub struct EscapeKey;

/// Every kind, and its count.
#[cfg_attr(rust_js, rust_js::type_map(map = "Vocabulary", keys = "Key", lookup = "ValueOf"))]
#[cfg_attr(rust_js, rust_js::types = "string")]
pub trait Kind: Copy {
    type Value;
}

impl Kind for EscapeKey {
    type Value = u32;
}

impl Kind for &'static str {
    type Value = String;
}

pub fn escape() -> EscapeKey {
    EscapeKey
}
`);
  writeFileSync(join(dir, "lib.rs"), `mod model;

pub use model::Kind as Sort;
pub use model::{EscapeKey, escape};
`);
  run([compiler, "--declarations", join(dir, "lib.rs"), "-o", join(dir, "lib.js")]);
  const declarations = readFileSync(join(dir, "lib.d.ts"), "utf8");
  expect(declarations).toContain('export type { Vocabulary, Key, ValueOf } from "./model.js";');
  expect(declarations).not.toContain("Sort");
  expect(readFileSync(join(dir, "lib.js"), "utf8")).not.toContain("Vocabulary");
  writeFileSync(join(dir, "consumer.ts"), `import { escape, type Key, type ValueOf, type Vocabulary } from "./lib.js";
export const key: Key = escape();
export const count: ValueOf<"escape-key"> = 3;
export const other: ValueOf<"anything"> = "text";
export const counted: Vocabulary["escape-key"] = 1;
// @ts-expect-error No such key.
export const unknown: Key = "enter";
`);
  expect(typeChecked(dir, "consumer.ts", checking)).toEqual([]);
});

test("a type map written otherwise, or of an impl that names no key, is refused", () => {
  const dir = fixture("declarations-type-maps-refused");
  writeFileSync(join(dir, "lib.rs"), `#[cfg_attr(rust_js, rust_js::type_map(map = "Vocabulary", lookup = "Of"))]
pub trait Reason {
    type Event;
}

pub struct Unnamed;

#[cfg_attr(rust_js, rust_js::type_map(map = "Others"))]
pub trait Other {
    type Event;
}

impl Other for Unnamed {
    type Event = u32;
}
`);
  const refused = Bun.spawnSync([compiler, "--declarations", join(dir, "lib.rs"), "-o", join(dir, "lib.js"), ...withWeb], { cwd: root, stderr: "pipe" });
  const said = refused.stderr.toString();
  expect([refused.exitCode === 0, said.includes("a lookup only with keys"), said.includes("is of a unit struct whose `#[rust_js::name = \"..\"]` is its key")]).toEqual([false, true, true]);
});

// A function's type parameter TypeScript keeps a caller's literals of,
// `#[rust_js::const_param]`; its default, a bound rustc resolved,
// `js::DefaultsTo<js::True>`; and the js crate's literal booleans, `js::True`
// and `js::False`, whose values are `true` and `false`, read as a `bool` by
// `js::boolean`, the value itself (ADR 1164, D and E).
test("a const type parameter, a default by its bound, and literal booleans", () => {
  const dir = fixture("declarations-literal-booleans");
  writeFileSync(join(dir, "lib.rs"), `/// What it's given, kept.
pub fn kept<#[cfg_attr(rust_js, rust_js::const_param)] G>(given: G) -> G {
    given
}

/// The same, of its own type.
pub fn pick<C>(cancelable: C) -> C
where
    C: js::Boolean + js::DefaultsTo<js::True>,
{
    cancelable
}

pub fn yes() -> js::True {
    js::True
}

pub fn no() -> js::False {
    js::False
}

/// Whether it may be canceled.
pub fn cancels<C: js::Boolean>(cancelable: C) -> &'static str {
    if !js::boolean(cancelable) { "commit" } else { "change" }
}

/// What a kind is, by whether it cancels: a conditional type, written.
#[cfg_attr(rust_js, rust_js::types = "C extends true ? \\"change\\" : \\"commit\\"")]
pub type Shape<C> = (C, &'static str);

pub fn shape<C: js::Boolean + Copy>(cancelable: C) -> Shape<C> {
    (cancelable, "shape")
}
`);
  run([compiler, "--declarations", join(dir, "lib.rs"), "-o", join(dir, "lib.js"), ...withWeb]);
  const declarations = readFileSync(join(dir, "lib.d.ts"), "utf8");
  expect(declarations).toContain("export function kept<const G>(given: G): G;");
  expect(declarations).toContain("export function pick<C extends boolean = true>(cancelable: C): C;");
  expect(declarations).toContain("export function yes(): true;");
  expect(declarations).toContain("export function no(): false;");
  expect(declarations).toContain('export type Shape<C> = C extends true ? "change" : "commit";');
  const js = readFileSync(join(dir, "lib.js"), "utf8");
  expect(js).toContain("export function yes() {\n  return true;\n}");
  expect(js).toContain('  return !cancelable ? "commit" : "change";');
  writeFileSync(join(dir, "consumer.ts"), `import { cancels, kept, no, pick, shape, yes } from "./lib.js";
export const reason: "escape-key" = kept({ reason: "escape-key" }).reason;
export const literal: true = yes();
export const other: false = no();
export const chosen: false = pick(false);
export const said: string = cancels(true);
export const changes: "change" = null as unknown as ReturnType<typeof shape<true>>;
// @ts-expect-error A literal \`false\` isn't \`true\`.
export const wrong: true = no();
`);
  expect(typeChecked(dir, "consumer.ts", checking)).toEqual([]);
  writeFileSync(join(dir, "run.mjs"), `import { cancels, yes } from "./lib.js";\nconsole.log(cancels(yes()), cancels(false));\n`);
  expect(run(["node", join(dir, "run.mjs")])).toBe("change commit\n");
});

test("a type map preserves written event aliases and a record's reason parameter", () => {
  const dir = fixture("declaration-type-map-record");
  const source = program.replace("impl Reason for EscapeKey {", `#[cfg_attr(rust_js, rust_js::types = "T | undefined")]
pub type OptionalEvent<T> = Option<T>;

#[cfg_attr(rust_js, rust_js::types = "\\"escape-key\\" | \\"trigger-press\\"")]
pub type DefaultReason = &'static str;

pub struct ChangeOptions<R: Reason = DefaultReason> {
    pub reason: Option<R>,
}

impl Reason for EscapeKey {`).replace("type Event = &'static KeyboardEvent;", "type Event = OptionalEvent<&'static KeyboardEvent>;");
  writeFileSync(join(dir, "lib.rs"), source);
  run([compiler, "--declarations", join(dir, "lib.rs"), "-o", join(dir, "lib.js"), ...withWeb]);
  const declarations = readFileSync(join(dir, "lib.d.ts"), "utf8");
  expect(declarations).toContain('  "escape-key": OptionalEvent<KeyboardEvent>;');
  expect(declarations).toContain("export interface ChangeOptions<R extends string = DefaultReason> {\n    reason?: R | undefined;");
});
