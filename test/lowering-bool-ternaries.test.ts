// A conditional of `bool`s one branch of which is a literal, `c ? b :
// false` and `c ? true : b`, is the `&&` or `||` it means, `c && b` and
// `c || b`, of an `if` or a `match` the program wrote as of one MIR made
// (ADR 0879, amended, the user's decision). Each test is a program of its
// own, compiled and run.

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

// React's isFragmentInstanceHostBoundary writes `supportsSingletons ? tag
// === HostSingleton : false`; the user's decision is the `&&` it means, of
// an `if` or a `match` alike: `return at != null && at < 3`. A branch of
// what isn't a `bool`, an `Option<bool>`'s `Some(false)`, keeps the
// conditional, as does a test that isn't a `bool`, a value's truth.
test("a written if or match of a bool literal branch is && or ||", async () => {
  const { js, lib } = await compiled(
    "bool-ternaries",
    `use js::Unknown;

pub fn small(s: &str) -> bool {
    match s.find('b') {
        Some(at) => at < 3,
        None => false,
    }
}

pub fn both(dev: bool, n: u32) -> bool {
    if dev { n > 2 } else { false }
}

#[cfg_attr(rust_js, rust_js::link_name = "globalThis.bigIt")]
#[allow(unused_variables)]
fn big_it(n: u32) -> bool {
    unreachable!()
}

pub fn called(dev: bool, n: u32) -> bool {
    if dev { big_it(n) } else { false }
}

pub fn picked(dev: bool, flags: &[bool]) -> bool {
    if dev { flags[1] } else { false }
}

pub fn either(dev: bool, n: u32) -> bool {
    if dev { true } else { n > 2 }
}

pub fn stored(dev: bool, n: u32, big: &dyn Fn(u32) -> bool) -> u32 {
    let wide = if dev { big(n) } else { false };
    let any = if n == 0 { true } else { dev };
    u32::from(wide) * 10 + u32::from(any)
}

pub fn kept(dev: bool, n: Option<bool>) -> Option<bool> {
    if dev { n } else { Some(false) }
}

pub fn truth(v: Option<&Unknown>, n: u32) -> bool {
    if js::truthy(v) { n > 2 } else { false }
}
`,
  );
  expect(js).toContain("  return at != null && at < 3;\n");
  expect(js).toContain("  return dev && n > 2;\n");
  expect(js).toContain("  return dev || n > 2;\n");
  expect(js).toContain("  return dev && $index(flags, 1);\n");
  expect(js).toContain("  return dev && globalThis.bigIt(n);\n");
  expect(js).toContain("  const wide = dev && big(n);\n");
  expect(js).toContain("  const any = n === 0 || dev;\n");
  expect(js).toContain("  return dev ? n : false;\n");
  expect(js).toContain("  return v ? n > 2 : false;\n");
  expect([lib.small("abc" as never), lib.small("aaab" as never), lib.small("x" as never)]).toEqual([true, false, false]);
  const big = (n: number) => n > 2;
  (globalThis as Record<string, unknown>).bigIt = big;
  for (const dev of [false, true]) {
    for (const n of [0, 3]) {
      expect([dev, n, lib.both(dev as never, n as never), lib.either(dev as never, n as never), lib.called(dev as never, n as never), lib.stored(dev as never, n as never, big as never)]).toEqual([
        dev,
        n,
        dev && n > 2,
        dev || n > 2,
        dev && n > 2,
        (dev && n > 2 ? 10 : 0) + (n === 0 || dev ? 1 : 0),
      ]);
    }
    expect(lib.picked(dev as never, [false, true] as never)).toBe(dev);
    expect([lib.kept(dev as never, true as never), lib.kept(dev as never, undefined as never)]).toEqual([dev ? true : false, dev ? undefined : false]);
  }
  expect([lib.truth(0 as never, 5 as never), lib.truth("x" as never, 5 as never), lib.truth("x" as never, 1 as never)]).toEqual([false, true, false]);
});

// A call that gives a `bool` is one, of another module's function too,
// which the module's own names don't say: React DOM's ChangeEventPlugin's
// `canUseDOM && isEventSupported('input') && (..)`, no `!!` of its first
// two (rd145).
test("a call of another module's function that gives a bool is one", async () => {
  const dir = fixture("bool-call-of-another-module");
  writeFileSync(
    join(dir, "lib.rs"),
    `mod support {
    pub fn is_supported(name: &str) -> bool {
        name.len() > 3
    }
}

pub mod plugin {
    use crate::support::is_supported;

    pub fn input_supported(ready: bool, name: &str, mode: f64) -> bool {
        ready && is_supported(name) && (mode < 1.0 || mode > 9.0)
    }
}
`,
  );
  run([compiler, join(dir, "lib.rs"), "-o", join(dir, "lib.js")]);
  const js = readFileSync(join(dir, "plugin.js"), "utf8");
  expect(js).toContain("  return ready && isSupported(name) && (mode < 1 || mode > 9);\n");
  const { inputSupported } = await import(join(dir, "plugin.js"));
  expect([inputSupported(true, "input", 0), inputSupported(true, "in", 0), inputSupported(true, "input", 5)]).toEqual([true, false, false]);
});
