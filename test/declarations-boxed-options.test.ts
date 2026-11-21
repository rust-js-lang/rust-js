// What a crate's declarations (ADR 0196) say of an `Option` whose payload
// may look like `None`, which ADR 0051 boxes where it's `Some`, `$some(v)`:
// what of the payload isn't `None`-like, and `@rust-js/std`'s `SomeNone`,
// the box of what is, as lowering's `boxed_payload` says (#842). A
// `Some(())` is `SomeNone`, never `undefined`, which is `None`'s.
// TypeScript that uses each is checked against it, and the JS gives what
// it says; a JS-made object of a nullable `Option<Option<T>>` field reads
// as Rust's states do.

import { expect, test } from "bun:test";
import { readFileSync, writeFileSync } from "node:fs";
import { join } from "node:path";
import { compiler, fixture, installStd, run } from "./support";
import { es2022, repoTypes, type Setting, typeChecked } from "./type-checked";

// What this file's checks of its declarations are made under (`typeChecked`).
const checking: Setting = { ...es2022, ...repoTypes };

test("an Option of what may look like None is declared its box, SomeNone", async () => {
  const dir = fixture("declarations-boxed-options");
  writeFileSync(join(dir, "lib.rs"), `pub struct Holder {
    pub unit: Option<()>,
    pub nested: Option<Option<u32>>,
}

pub fn unit(on: bool) -> Option<()> {
    if on { Some(()) } else { None }
}

pub fn nested(n: u32) -> Option<Option<u32>> {
    match n {
        0 => None,
        1 => Some(None),
        _ => Some(Some(n)),
    }
}

pub fn given(x: Option<()>) -> bool {
    x.is_some()
}

pub fn holder() -> Holder {
    Holder { unit: Some(()), nested: Some(None) }
}
`);
  run([compiler, "--declarations", join(dir, "lib.rs"), "-o", join(dir, "lib.js")]);
  installStd(dir);
  const declarations = readFileSync(join(dir, "lib.d.ts"), "utf8");
  expect(declarations).toContain('import type { SomeNone } from "@rust-js/std";');
  expect(declarations).toContain("export function unit(on: boolean): SomeNone | null | undefined;");
  expect(declarations).toContain("export function nested(n: number): number | SomeNone | null | undefined;");
  expect(declarations).toContain("export function given(x?: SomeNone | null | undefined): boolean;");
  expect(declarations).toContain("    unit?: SomeNone | undefined;\n    nested?: number | SomeNone | undefined;");
  writeFileSync(join(dir, "use.ts"), `import type { SomeNone } from "@rust-js/std";
import { given, holder, nested, unit } from "./lib.js";
export const some: SomeNone | null | undefined = unit(true);
export const inner: number | SomeNone | null | undefined = nested(3);
export const passed: boolean = given(unit(true));
export const field: SomeNone | undefined = holder().unit;
`);
  expect(typeChecked(dir, "use.ts", checking)).toEqual([]);
  writeFileSync(join(dir, "wrong.ts"), `import { unit } from "./lib.js";
export const wrong: undefined = unit(true);
`);
  expect(typeChecked(dir, "wrong.ts", checking)).toEqual([expect.stringContaining("error TS2322")]);
  const lib = await import(join(dir, "lib.js"));
  expect([lib.unit(true), lib.unit(false), lib.nested(0), lib.nested(1), lib.nested(5)]).toEqual([
    { $someNone: 0 },
    undefined,
    undefined,
    { $someNone: 0 },
    5,
  ]);
  expect([lib.given(lib.unit(true)), lib.given(undefined), lib.given(null)]).toEqual([true, false, false]);
});

// #836's field, a pub struct's nullable `Option<Option<T>>` of a `T` that
// may look like `None`, once refused, `field?: T | null` not declaring the
// box: declared with it, and an object JS makes, of `undefined`, `null`, a
// `T` or the box, reads as Rust's `None`, `Some(None)`, `Some(Some(t))` and
// `Some(Some(None))` (#842).
test("a pub struct's nullable Option of an Option of what may look like None is declared, and read of what JS makes", async () => {
  const dir = fixture("declarations-boxed-nested-field");
  writeFileSync(join(dir, "lib.rs"), `pub struct Init {
    #[cfg_attr(rust_js, rust_js::nullable)]
    pub mid: Option<Option<Option<u32>>>,
}

pub fn state(init: &Init) -> u32 {
    match init.mid {
        None => 0,
        Some(None) => 1,
        Some(Some(None)) => 2,
        Some(Some(Some(n))) => n + 10,
    }
}

pub fn make(n: u32) -> Init {
    let mid = match n {
        0 => None,
        1 => Some(None),
        2 => Some(Some(None)),
        _ => Some(Some(Some(n))),
    };
    Init { mid }
}
`);
  run([compiler, "--declarations", join(dir, "lib.rs"), "-o", join(dir, "lib.js")]);
  installStd(dir);
  expect(readFileSync(join(dir, "lib.d.ts"), "utf8")).toContain("    mid?: number | SomeNone | null | undefined;");
  const lib = await import(join(dir, "lib.js"));
  expect([{}, { mid: undefined }, { mid: null }, { mid: 5 }, { mid: { $someNone: 0 } }].map((made) => lib.state(made))).toEqual([0, 0, 1, 15, 2]);
  expect([0, 1, 2, 7].map((n) => lib.state(lib.make(n)))).toEqual([0, 1, 2, 17]);
});

// What looks like `None` is seen through a `Pin` (#832): a nullable nested
// field of a `Pin<Box<()>>`, whose `()` is `undefined`, boxes its
// `Some(Some(..))`, so its three states stay apart where it's made and read
// (#842).
test("a nullable Option of an Option of a Pin of a () keeps its three states", async () => {
  const dir = fixture("declarations-boxed-pin-field");
  writeFileSync(join(dir, "lib.rs"), `use std::pin::Pin;

pub struct Init {
    #[cfg_attr(rust_js, rust_js::nullable)]
    pub mid: Option<Option<Pin<Box<()>>>>,
}

pub fn state(init: &Init) -> u32 {
    match &init.mid {
        None => 0,
        Some(None) => 1,
        Some(Some(_)) => 2,
    }
}

pub fn make(n: u32) -> Init {
    let mid = match n {
        0 => None,
        1 => Some(None),
        _ => Some(Some(Box::pin(()))),
    };
    Init { mid }
}
`);
  run([compiler, "--declarations", join(dir, "lib.rs"), "-o", join(dir, "lib.js")]);
  installStd(dir);
  const lib = await import(join(dir, "lib.js"));
  expect([0, 1, 2].map((n) => lib.state(lib.make(n)))).toEqual([0, 1, 2]);
});
