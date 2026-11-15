// A plain `Option`, a field or a parameter, is omittable: a field's `None`
// left out where its struct is made, `point?: T`, a trailing parameter's
// `None` left out by a caller, `value?: T`. `#[rust_js::present]` makes the
// other way, a present slot: the field's key written, `{ value: 3, point:
// undefined }`, `point: T | undefined`, as `Object.hasOwn` and
// `Reflect.ownKeys` tell; and `#[rust_js::present(value)]` a parameter a
// caller gives, `value: T | null | undefined` (ADR 1161, #1845).
// `#[rust_js::optional]` is the one axis's other way, a cell's or a nullable
// field's key left out, each refused where it's its field's own default;
// and a binding's result whose `None` is `undefined` alone is
// `#[rust_js::none_undefined]`, beside `#[rust_js::nullable]`.

import { beforeAll, expect, test } from "bun:test";
import { readFileSync, writeFileSync } from "node:fs";
import { join } from "node:path";
import { runSync } from "./child";
import { buildReact, compiler, fixture, root, run, target } from "./support";
import { underLoad } from "../scripts/timing";

beforeAll(buildReact, 600_000);

const js = ["--", "--crate-type=lib", "--edition=2024", "--extern", `js=${join(target, "libjs.rmeta")}`, "-L", target];

test("a present field's key is written and declared there, a plain one's left out", async () => {
  const dir = fixture("present-slots");
  writeFileSync(
    join(dir, "lib.rs"),
    `pub struct Sample {
    pub value: f64,
    #[cfg_attr(rust_js, rust_js::present)]
    pub point: Option<f64>,
    pub plain: Option<f64>,
}

pub fn sample(point: Option<f64>) -> Sample {
    Sample { value: 3.0, point, plain: None }
}

pub fn without() -> Sample {
    Sample { value: 3.0, point: None, plain: None }
}

pub fn plain_param(value: Option<&str>) -> bool {
    value.is_some()
}

#[cfg_attr(rust_js, rust_js::present(value))]
pub fn is_iso_date(value: Option<&str>) -> bool {
    value.is_some_and(|v| v.len() == 10)
}
`,
  );
  run([compiler, "--declarations", join(dir, "lib.rs"), "-o", join(dir, "lib.js"), ...js]);
  const out = readFileSync(join(dir, "lib.js"), "utf8");
  const declarations = readFileSync(join(dir, "lib.d.ts"), "utf8");
  expect(out).toContain("return { value: 3, point: undefined };");
  expect(declarations).toContain("    point: number | undefined;\n");
  expect(declarations).toContain("    plain?: number | undefined;\n");
  expect(declarations).toContain("export function plainParam(value?: string | null | undefined): boolean;");
  expect(declarations).toContain("export function isIsoDate(value: string | null | undefined): boolean;");
  const lib = await import(join(dir, "lib.js"));
  expect(Reflect.ownKeys(lib.without())).toEqual(["value", "point"]);
  expect([Object.hasOwn(lib.sample(undefined), "point"), lib.sample(2).point]).toEqual([true, 2]);
  expect([lib.isIsoDate("2026-11-15"), lib.isIsoDate(undefined)]).toEqual([true, false]);
}, underLoad(120_000));

const refused: [string, string, string][] = [
  [
    "a cell's key",
    `pub struct S {\n    #[cfg_attr(rust_js, rust_js::present)]\n    pub c: std::cell::Cell<Option<u32>>,\n}\npub fn s() -> S {\n    S { c: std::cell::Cell::new(None) }\n}\n`,
    "a cell's or a `#[rust_js::nullable]` field's key is written already",
  ],
  [
    "a nullable field's key",
    `pub struct S {\n    #[cfg_attr(rust_js, rust_js::nullable)]\n    #[cfg_attr(rust_js, rust_js::present)]\n    pub n: Option<u32>,\n}\npub fn s() -> S {\n    S { n: None }\n}\n`,
    "a cell's or a `#[rust_js::nullable]` field's key is written already",
  ],
  [
    "a field that isn't an Option",
    `pub struct S {\n    #[cfg_attr(rust_js, rust_js::present)]\n    pub n: u32,\n}\npub fn s() -> S {\n    S { n: 1 }\n}\n`,
    "`#[rust_js::present]` is of a plain `Option` field",
  ],
  [
    "a parameter that isn't an Option",
    `#[cfg_attr(rust_js, rust_js::present(n))]\npub fn f(n: u32) -> u32 {\n    n\n}\n`,
    "`#[rust_js::present(n)]` is of a parameter of its own, an `Option`",
  ],
  [
    "a function's optional",
    `#[cfg_attr(rust_js, rust_js::link_name = "globalThis.event")]\n#[cfg_attr(rust_js, rust_js::optional)]\nfn event() -> Option<u32> {\n    unreachable!()\n}\npub fn e() -> Option<u32> {\n    event()\n}\n`,
    "is `#[rust_js::none_undefined]`",
  ],
];

for (const [name, source, message] of refused) {
  test(`present and optional are refused of ${name}`, () => {
    const dir = fixture("present-refused");
    writeFileSync(join(dir, "lib.rs"), source);
    const p = runSync([compiler, join(dir, "lib.rs"), "-o", join(dir, "lib.js"), ...js], root, 60_000);
    expect([p.code === 0, p.stderr]).toEqual([false, expect.stringContaining(message)]);
  }, underLoad(60_000));
}
