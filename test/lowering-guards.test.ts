// What the compiler writes of a `match` arm's guard and the bindings its
// pattern makes for it (ADR 0751). Each test is a program of its own,
// compiled and run.

import { beforeAll, expect, test } from "bun:test";
import { readFileSync, writeFileSync } from "node:fs";
import { join } from "node:path";
import { buildReact, compiler, fixture, run, target } from "./support";

beforeAll(buildReact, 600_000);

/** `rust` compiled in a fixture of `name`, against the js crate: its JS,
 * and its module. */
async function compiled(name: string, rust: string): Promise<{ js: string; lib: Record<string, (...args: never[]) => unknown> }> {
  const dir = fixture(name);
  writeFileSync(join(dir, "lib.rs"), rust);
  run([compiler, join(dir, "lib.rs"), "-o", join(dir, "lib.js"), "--", "--extern", `js=${join(target, "libjs.rmeta")}`, "-L", target]);
  return { js: readFileSync(join(dir, "lib.js"), "utf8"), lib: await import(join(dir, "lib.js")) };
}

// React's getArrayKind: `Kind::Array(&[first, _]) if matches!(js::classify(first),
// Kind::String(_)) => { kind = Entries; }`. The guard reads `first`, through
// a reference of its own; the arm's body, which has its own `first`, reads
// none. Upstream writes `typeof value[0] === 'string'` and no `first`: the
// arm's binding nothing reads isn't written, `const first = value[0];`.
test("a binding only an arm's guard reads isn't written in the arm", async () => {
  const { js, lib } = await compiled("guard-only-binding", `use js::{Kind, Unknown};

pub fn kind(value: Option<&Unknown>) -> &'static str {
    match js::classify(value) {
        Kind::Array(&[first, _]) if matches!(js::classify(first), Kind::String(_)) => "entry",
        Kind::Array(&[_, second]) if matches!(js::classify(second), Kind::Number(_)) => "pair",
        _ => "other",
    }
}
`);
  expect(js).toContain('typeof value[0] === "string"');
  expect(js).not.toContain("const first");
  expect([["a", 1], [1, 2], [1, "b"], "x", [1, 2, 3]].map((v) => lib.kind(v as never))).toEqual(["entry", "pair", "other", "other", "other"]);
});
