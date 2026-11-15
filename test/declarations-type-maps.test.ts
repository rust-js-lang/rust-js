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
  expect(readFileSync(join(dir, "lib.js"), "utf8")).toContain('export function pressed() {\n  return "trigger-press";\n}');
  writeFileSync(join(dir, "consumer.ts"), `import { pressed, type ChangeReason, type EventOf, type ReasonVocabulary } from "./lib.js";
export const reason: "trigger-press" = pressed();
export const reasons: ChangeReason[] = ["trigger-press", "escape-key"];
export const key: EventOf<"escape-key"> = new KeyboardEvent("keydown");
export const press: EventOf<"trigger-press"> = new MouseEvent("click");
export const other: EventOf<"anything"> = new Event("other");
export const vocabulary: ReasonVocabulary["escape-key"] = new KeyboardEvent("keydown");
// @ts-expect-error No such reason.
export const unknown: ChangeReason = "hover";
// @ts-expect-error An escape key's event is a key's.
export const wrong: EventOf<"escape-key"> = new MouseEvent("click");
`);
  expect(typeChecked(dir, "consumer.ts")).toEqual([]);
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
