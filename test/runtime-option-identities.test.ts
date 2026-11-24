// The identities of the runtime's `Option` helpers that lowering folds
// when it reads a value it just made (#2200's class-5 reasons, #2334):
// `some_value` (std_types/option.rs) writes `$someValue($some(x))` as `x`,
// `$someValue($someGet(map, key))` as `map.get(key)`, and
// `$someValue($someDefined(v))` as `v`; `held_of_boxed`
// (mir/nested_options.rs) writes the payload held of a box it can read
// whole. Each fold is right whoever wrote the call, as each identity is of
// the helpers' own definitions, which these hold.
import { expect, test } from "bun:test";
import { $hashedMap, $OrdMap, $OrdTree, $some, $someDefined, $someGet, $someValue } from "../std/runtime.js";

// A `Some`'s payload of each shape: plain values, what looks like `None`
// (`undefined`, `null`) and the boxes of `Option`s nested in one another.
const payloads = [0, 1, "", "text", false, { a: 1 }, [1, 2], undefined, null, $some(undefined), $some(null), $some($some(null)), $some($some(undefined))];

test("$someValue of $some(x) is x", () => {
  for (const x of payloads) expect([x, $someValue($some(x))]).toEqual([x, x]);
  // Of a value no box is made for, the value itself.
  for (const x of [5, "s", { a: 2 }]) expect($someValue($some(x))).toBe(x);
});

test("$someValue of $someDefined(v) is v, of a value there", () => {
  for (const v of payloads.filter((v) => v !== undefined)) expect([v, $someValue($someDefined(v))]).toEqual([v, v]);
  expect($someDefined(undefined)).toBeUndefined();
});

test("$someValue of $someGet(map, key) is map.get(key), of a key there, in each map", () => {
  const KHash = { hash: (key: { a: number }, cell: { value: { write(bytes: number[]): void } }) => cell.value.write([key.a]) };
  const Hashed = $hashedMap(KHash, (k: { a: number }, held: { a: number }) => k.a === held.a);
  const cmp = (a: number, b: number) => (a < b ? -1 : a > b ? 1 : 0);
  // Each a map the runtime's `$someGet` reads, `get` and all.
  type Got = { get(key: unknown): unknown };
  const maps: [string, Got, unknown[]][] = [
    ["Map", new Map<unknown, unknown>([["k", 1], ["n", null], ["b", $some(null)]]), ["k", "n", "b"]],
    ["$hashedMap", new Hashed([[{ a: 1 }, "one"], [{ a: 2 }, null]]), [{ a: 1 }, { a: 2 }]],
    ["$OrdMap", new $OrdMap(cmp, [[1, "one"], [2, null]]), [1, 2]],
    ["$OrdTree", new $OrdTree(cmp, [[1, "one"], [2, null], [3, $some(undefined)]]), [1, 2, 3]],
  ];
  for (const [kind, map, keys] of maps) {
    for (const key of keys) expect([kind, key, $someValue($someGet(map, key))]).toEqual([kind, key, map.get(key)]);
  }
});

test("a box held as its payload: the forms held_of_boxed folds are $someValue's", () => {
  // `held == none ? undefined : $some(held)`, held, is `held` whether or not
  // it's `none`: `$someValue` of `$some(held)` is `held`, and `none`'s box is
  // `undefined`, which holds as `none`.
  for (const none of [undefined, null]) {
    for (const held of [none, 0, "x", { a: 1 }]) {
      const boxed = held == none ? undefined : $some(held);
      const general = boxed == null ? none : $someValue(boxed);
      expect([none, held, general]).toEqual([none, held, held]);
    }
  }
  // `$some(inner)` of an `Option` of an `Option`, `inner ?? null`: its
  // `$someValue` read as `null` where it's `None`.
  for (const inner of [undefined, null, 0, { a: 1 }]) expect($someValue($some(inner)) ?? null).toEqual(inner ?? null);
  // A constant that isn't `null`, itself: no box is made of it.
  for (const constant of [0, 7, "s", true]) expect($someValue(constant)).toBe(constant);
});
