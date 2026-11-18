// An untagged enum each of whose variants holds a record, flattened into
// its parent (#1818, ADR 1071, amended): a UI kit's numeric keyboard event,
// a key and its modifiers together with a composition event, direct or
// wrapped by a framework, upstream's `{ key; shiftKey?; altKey? } &
// Parameters<typeof isComposing>[0]`. What its declarations say, that
// TypeScript checks a caller by them and JS's own objects are told apart as
// Rust's are, and what's refused. The corpus's
// `flattened_untagged_records` runs it against native Rust.

import { beforeAll, expect, test } from "bun:test";
import { readFileSync, writeFileSync } from "node:fs";
import { join } from "node:path";
import { runSync } from "./child";
import { buildReact, compiler, fixture, root, target } from "./support";
import { typeChecked } from "./type-checked";

beforeAll(buildReact, 600_000);

const bindings = ["--", "--extern", `webapi=${join(target, "libwebapi.rmeta")}`, "--extern", `js=${join(target, "libjs.rmeta")}`, "-L", target];

// The kit's own types (vercel-ui-sdk-in-rust's events.rs), the event that
// flattens them beside.
const events = `use webapi::Event;

/// Optional composition fields shared by keyboard/input events and framework events.
#[derive(Default)]
pub struct CompositionFields {
    pub is_composing: Option<bool>,
    pub key_code: Option<f64>,
}

#[cfg_attr(rust_js, rust_js::untagged)]
pub enum NativeCompositionEvent<'a> {
    Event(&'a Event),
    Fields(&'a CompositionFields),
}

pub struct WrappedCompositionEvent<'a> {
    #[cfg_attr(rust_js, rust_js::test)]
    pub native_event: NativeCompositionEvent<'a>,
}

#[cfg_attr(rust_js, rust_js::untagged)]
pub enum CompositionInput<'a> {
    Wrapped(WrappedCompositionEvent<'a>),
    #[cfg_attr(rust_js, rust_js::otherwise)]
    Direct(&'a CompositionFields),
}

pub struct NumberKeyEvent<'a> {
    pub key: &'a str,
    pub shift_key: Option<bool>,
    pub alt_key: Option<bool>,
    #[cfg_attr(rust_js, rust_js::flatten)]
    pub composition: CompositionInput<'a>,
}

// Every payload is an object, never null or undefined.
unsafe impl js::Defined for CompositionInput<'_> {}

/// Whether an input method is composing text. Safari's legacy key code 229
/// counts even when is_composing is false, so the final candidate is left alone.
pub fn is_composing(event: &CompositionInput) -> bool {
    let event = js::unknown(event);
    let key = if js::has(event, "nativeEvent") {
        js::get(event, "nativeEvent")
    } else {
        Some(event)
    };
    // SAFETY: the input union guarantees an object. Its two fields are optional.
    // Like the upstream assertion, a plain Event need not declare either one.
    let key: &CompositionFields = unsafe { js::cast(key) };
    key.is_composing == Some(true) || key.key_code == Some(229.0)
}

/// Whether a framework wrapped the event, told by its \`nativeEvent\`.
pub fn wrapped(event: &NumberKeyEvent) -> bool {
    matches!(event.composition, CompositionInput::Wrapped(_))
}

pub fn handles(event: &NumberKeyEvent) -> bool {
    !is_composing(&event.composition) && event.key.len() == 1 && event.alt_key != Some(true)
}
`;

test("an untagged enum of records flattened is its parent's keys, declared an intersection, told apart on the parent", async () => {
  const dir = fixture("flattened-untagged");
  writeFileSync(join(dir, "lib.rs"), events);
  const ran = runSync([compiler, "--declarations", join(dir, "lib.rs"), "-o", join(dir, "lib.js"), ...bindings], root, 600_000);
  expect(ran.code, ran.stderr).toBe(0);
  const declarations = readFileSync(join(dir, "lib.d.ts"), "utf8");
  expect(declarations).toContain("export type CompositionInput = WrappedCompositionEvent | CompositionFields;");
  expect(declarations).toContain(`export type NumberKeyEvent = {
    key: string;
    shiftKey?: boolean | undefined;
    altKey?: boolean | undefined;
} & CompositionInput;`);
  const js = readFileSync(join(dir, "lib.js"), "utf8");
  // Lent whole, `&event.composition` is the event itself.
  expect(js).toContain("return !isComposing(event) && ");
  // A caller gives one object, either record's keys beside the key's.
  writeFileSync(
    join(dir, "use.ts"),
    `import { handles, type NumberKeyEvent } from "./lib.js";
const direct: NumberKeyEvent = { key: "1", shiftKey: true, isComposing: false };
const wrapped: NumberKeyEvent = { key: "2", nativeEvent: { keyCode: 229 } };
const native: NumberKeyEvent = { key: "3", nativeEvent: new Event("keydown") };
export const all: boolean[] = [direct, wrapped, native].map(handles);
// @ts-expect-error: a key is text
export const wrong: NumberKeyEvent = { key: 4 };
`,
  );
  const checked = typeChecked(dir, "use.ts", { allowJs: true, skipLibCheck: true, lib: ["esnext", "dom"] });
  expect(checked).toEqual([]);
  // JS's own objects, as a framework gives them, told apart as Rust's are.
  const { handles, isComposing, wrapped } = await import(join(dir, "lib.js"));
  expect(handles({ key: "1" })).toBe(true);
  expect(handles({ key: "1", isComposing: true })).toBe(false);
  expect(handles({ key: "1", keyCode: 229 })).toBe(false);
  expect(handles({ key: "1", altKey: true })).toBe(false);
  expect(handles({ key: "1", nativeEvent: { keyCode: 229 } })).toBe(false);
  expect(handles({ key: "1", nativeEvent: { isComposing: false } })).toBe(true);
  expect(isComposing({ key: "1", nativeEvent: new Event("compositionupdate") })).toBe(false);
  expect([wrapped({ key: "1", nativeEvent: new Event("keydown") }), wrapped({ key: "1", nativeEvent: {} }), wrapped({ key: "1", keyCode: 229 })]).toEqual([true, true, false]);
});

/** What the compiler says of `rust`, beside the kit's types, which it refuses. */
function refused(name: string, rust: string): string {
  const dir = fixture(name);
  writeFileSync(join(dir, "lib.rs"), rust);
  const ran = runSync([compiler, join(dir, "lib.rs"), "-o", join(dir, "lib.js"), ...bindings], root, 600_000);
  expect(ran.code).not.toBe(0);
  return ran.stderr;
}

// What can't be the parent's one object: a key the parent has of its own,
// a record's or the test member a variant's told by, which the parent's
// would answer; and the event written whole, which would leave the other
// record's keys. And each way its reference payload's copy, its keys spread
// into the parent, could be told from its referent, as a flattened
// reference is refused (#1819): a `Cell` of its own, its address compared,
// a `&mut`.
test("an untagged enum of records flattened is refused where its parent couldn't be its one object", () => {
  const shadowed = (field: string) => events.replace("    pub alt_key: Option<bool>,\n", `    pub alt_key: Option<bool>,\n    pub ${field}: Option<f64>,\n`);
  expect(refused("flattened-untagged-key", shadowed("key_code"))).toContain(
    "rust-js does not support flattening `CompositionInput<'a>`, whose `keyCode` is a key of its parent's own, which its untagged test would read yet",
  );
  expect(refused("flattened-untagged-test-member", shadowed("native_event"))).toContain(
    "flattening `CompositionInput<'a>`, whose `nativeEvent` is a key of its parent's own",
  );
  expect(
    refused(
      "flattened-untagged-written",
      `${events}pub fn redirect<'a>(event: &mut NumberKeyEvent<'a>, fields: &'a CompositionFields) {
    event.composition = CompositionInput::Direct(fields);
}
`,
    ),
  ).toContain("rust-js does not support reading flattened props whole");
  expect(
    refused(
      "flattened-untagged-cell",
      events.replace("    pub key_code: Option<f64>,\n}", "    pub key_code: Option<f64>,\n    pub seen: std::cell::Cell<u32>,\n}").replace("#[derive(Default)]\n", ""),
    ),
  ).toContain(
    "flattening `&'a CompositionFields`, of a type with a `Cell` or `RefCell` of its own, which its parent's object would hold a copy of, so a write through either wouldn't be seen through the other",
  );
  expect(
    refused(
      "flattened-untagged-compared",
      `${events}pub fn same(event: &NumberKeyEvent, fields: &CompositionFields) -> bool {
    match &event.composition {
        CompositionInput::Direct(direct) => std::ptr::eq(*direct, fields),
        CompositionInput::Wrapped(_) => false,
    }
}
`,
    ),
  ).toContain("flattening `&'a CompositionFields`, whose `CompositionFields` is changed in place or its address compared, which its parent's object, a copy of it, would answer apart from it");
  // One of a JS object, an `Event`, which an `instanceof` would tell, as
  // its parent isn't one.
  expect(
    refused(
      "flattened-untagged-js-object",
      `${events}pub struct Keyed<'a> {
    pub key: &'a str,
    #[cfg_attr(rust_js, rust_js::flatten)]
    pub native: NativeCompositionEvent<'a>,
}
`,
    ),
  ).toContain("flattening `NativeCompositionEvent<'a>`: a flattened field is");
  // A reference to one, which its parent's object would copy the record of.
  expect(refused("flattened-untagged-reference", events.replace("    pub composition: CompositionInput<'a>,", "    pub composition: &'a CompositionInput<'a>,").replace("is_composing(&event.composition)", "is_composing(event.composition)"))).toContain(
    "flattening `&'a CompositionInput<'a>`: a flattened field is",
  );
  expect(
    refused("flattened-untagged-mut", events.replace("    Direct(&'a CompositionFields),", "    Direct(&'a mut CompositionFields),").replace("CompositionInput::Direct(fields) => fields,", "CompositionInput::Direct(fields) => &**fields,")),
  ).toContain("flattening `&'a mut CompositionFields`, a `&mut`");
});
