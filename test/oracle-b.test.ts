// Oracle B's checker (test/generated-calls.ts `jsConforms`, ADR 1204): each
// JS shape it holds a value to is a row of the guide's types table, found
// there as it's written, so the checker mirrors the guide's contract, not
// the compiler's representation code; and of values of each shape, right
// and wrong, it says what the guide's rows do. The export net runs it,
// nightly; this file holds it on each push.
import { expect, test } from "bun:test";
import { readFileSync } from "node:fs";
import { join } from "node:path";
import { jsConforms, shapeRows, type NoneForm, type Shaped, type Tree } from "./generated-calls";
import { root } from "./support";

const conforms = new Function(`${jsConforms}\nreturn conforms;`)() as (
  v: unknown, t: Tree, none: NoneForm, types: Record<string, Shaped>, count: { checked: number; unchecked: number },
) => string | null;

test("each shape the checker holds a value to is a row of the guide's types table", () => {
  const guide = readFileSync(join(root, "docs/guide/interop/types.md"), "utf8");
  const missing = shapeRows.filter(([rust, js]) => !guide.includes(`\n| ${rust} | ${js} |`)).map(([rust]) => rust);
  expect(missing).toEqual([]);
});

const int = (rust: string): Tree => ({ kind: "int", rust });
const opt = (of: Tree): Tree => ({ kind: "option", of });
const point: Tree = { kind: "named", crate: "dep", rust: ["Point"] };
const types: Record<string, Shaped> = {
  "dep::Point": { fields: [{ js: "x", ty: int("u32") }, { js: "label", ty: opt({ kind: "str", owned: true }) }, { js: "seen", ty: opt(int("u8")), none: "null" }] },
  "dep::Meters": { transparent: { kind: "float", rust: "f64" } },
};
const said = (v: unknown, t: Tree, none: NoneForm = "plain") => {
  const count = { checked: 0, unchecked: 0 };
  return { problem: conforms(v, t, none, types, count), ...count };
};

test("a value of its record's shape conforms, and one of another doesn't, though both print alike", () => {
  expect(said(5, int("u32")).problem).toBeNull();
  expect(said(5n, int("u64")).problem).toBeNull();
  expect(said(5, int("u64")).problem).toBe("the value: number, not u64's BigInt");
  expect(said(5n, int("usize")).problem).toBe("the value: bigint, not usize's integer number");
  expect(said(256, int("u8")).problem).toBe("the value: number, not u8's range");
  expect(said(4294967295, int("usize")).problem).toBeNull();
  expect(said(0.1, { kind: "float", rust: "f32" }).problem).toBe("the value: number, not an f32, rounded by Math.fround");
  expect(said(new Uint32Array([1]), { kind: "vec", of: int("u32"), borrowed: false }).problem).toBe("the value: Uint32Array, not an array");
  expect(said([1, 2], { kind: "tuple", items: [int("u8")] }).problem).toBe("the value: an array, not an array of 1");
  expect(said("ab", { kind: "char" }).problem).toBe("the value: string, not a string of one character");
  expect(said(null, opt(int("u8"))).problem).toBe("the value: null, not u8's integer number");
  expect(said(null, opt(int("u8")), "null").problem).toBeNull();
  expect(said(undefined, opt(int("u8")), "null").problem).toBe("the value: undefined, not u8's integer number");
});

test("a struct is its fields' object, each None as its field's form says; a transparent one its field", () => {
  expect(said({ x: 1, seen: null }, point).problem).toBeNull();
  expect(said({ x: 1, label: "a", seen: 3 }, point).problem).toBeNull();
  // A plain field's `None` not known where it's made: a key of `undefined` (ADR 0280).
  expect(said({ x: 1, label: undefined, seen: null }, point).problem).toBeNull();
  expect(said({ x: 1 }, point).problem).toBe("the value.seen: no key, not its None's null");
  expect(said({ x: 1n, seen: null }, point).problem).toBe("the value.x: bigint, not u32's integer number");
  expect(said([1], point).problem).toBe("the value: an array, not an object of Point's fields");
  expect(said(2.5, { kind: "named", crate: "dep", rust: ["Meters"] }).problem).toBeNull();
  expect(said({ value: 2.5 }, { kind: "named", crate: "dep", rust: ["Meters"] }).problem).toBe("the value: Object, not f64's number");
});

// Of each enum form as the records say it, a variant's JS: a tagged one's
// tag or object of its tag, a declared tag's object, an untagged one's
// payload, a generic one's of what it's given (#2481's enums).
test("an enum's value is one of its variants in its form", () => {
  const enums: Record<string, Shaped> = {
    "dep::Shape": { variants: { form: "tagged", key: "TAG", variants: [
      { rust: "Empty", tag: { kind: "str", value: "Empty" } },
      { rust: "Square", tag: { kind: "str", value: "Square" }, fields: [{ js: "_0", ty: int("u32") }] },
    ] } },
    "dep::Job": { variants: { form: "declared", key: "status", variants: [
      { rust: "Waiting", tag: { kind: "str", value: "Waiting" } },
      { rust: "Done", tag: { kind: "str", value: "Done" }, fields: [{ js: "value", ty: int("u64") }] },
    ] } },
    "dep::Size": { variants: { form: "untagged", key: "TAG", variants: [
      { rust: "Named", tag: { kind: "str", value: "Named" }, fields: [{ js: "_0", ty: { kind: "str", owned: true } }] },
      { rust: "Exact", tag: { kind: "str", value: "Exact" }, fields: [{ js: "_0", ty: int("u32") }] },
    ] } },
    "dep::Held": { variants: { form: "tagged", key: "TAG", variants: [
      { rust: "One", tag: { kind: "str", value: "One" }, fields: [{ js: "_0", ty: { kind: "param", name: "T", index: 0 } }] },
    ] } },
  };
  const of = (rust: string, args?: Tree[]): Tree => ({ kind: "named", crate: "dep", rust: [rust], ...(args ? { args } : {}) });
  const says = (v: unknown, t: Tree) => conforms(v, t, "plain", enums, { checked: 0, unchecked: 0 });
  expect(says("Empty", of("Shape"))).toBeNull();
  expect(says({ TAG: "Square", _0: 3 }, of("Shape"))).toBeNull();
  expect(says({ TAG: "Empty" }, of("Shape"))).toBe('the value.TAG: "Empty", not a tag of Shape\'s');
  expect(says({ TAG: "Square", _0: 3n }, of("Shape"))).toBe("the value._0: bigint, not u32's integer number");
  expect(says({ status: "Waiting" }, of("Job"))).toBeNull();
  expect(says("Waiting", of("Job"))).toBe("the value: string, not one of Job's variants, its tag or an object of its status");
  expect(says(7, of("Size"))).toBeNull();
  expect(says(true, of("Size"))).toBe("the value: boolean, not one of Size's variants' payloads");
  expect(says({ TAG: "One", _0: 5n }, of("Held", [int("u64")]))).toBeNull();
  expect(says({ TAG: "One", _0: 5 }, of("Held", [int("u64")]))).toBe("the value._0: number, not u64's BigInt");
});

test("a shape the records can't decide is counted unchecked, never passed unsaid", () => {
  expect(said({ TAG: "A" }, { kind: "named", crate: "dep", rust: ["Shape"] })).toEqual({ problem: null, checked: 0, unchecked: 1 });
  expect(said(undefined, opt(opt(int("u8"))))).toEqual({ problem: null, checked: 0, unchecked: 1 });
  expect(said({ value: 1 }, { kind: "ref", of: int("u8"), mutable: true })).toEqual({ problem: null, checked: 0, unchecked: 1 });
  expect(said(1, { kind: "other", rust: "HashMap<u8, u8>" })).toEqual({ problem: null, checked: 0, unchecked: 1 });
});
