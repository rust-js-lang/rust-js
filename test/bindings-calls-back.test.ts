// A binding whose JS may call the crate's exported functions back says so,
// `#[rust_js::calls_back]` (#2119): what its call runs is then any function
// JS can call, so a thread-local whose initializer calls it is made where
// Rust makes it, at its first use, never as its module loads, when the
// exports its JS calls aren't there yet. One that doesn't say so is taken
// to call none back, the default Tu decided (ADR 0262's Consequences).
// Native Rust can't run the binding's JS, so the order is Rust's as its
// `thread_local!` defines it: made at the first `with`, not before.

import { beforeAll, expect, test } from "bun:test";
import { readFileSync, writeFileSync } from "node:fs";
import { join } from "node:path";

import { runSync } from "./child";
import { buildReact, compiler, fixture, target } from "./support";

beforeAll(buildReact, 600_000);

const externs = ["--", "--extern", `js=${join(target, "libjs.rmeta")}`, "-L", target];

/** The program, its binding marked as `mark` says. */
const program = (mark: string) => `use std::cell::Cell;

#[cfg_attr(rust_js, rust_js::link_name = "globalThis.reenter")]
${mark}
fn reenter() -> f64 {
    unreachable!()
}

thread_local! {
    static COUNT: Cell<u32> = const { Cell::new(0) };
    static FIRST: f64 = reenter();
}

/// What JS's \`reenter\` calls back: it counts, as state read after.
pub fn bump() -> u32 {
    COUNT.with(|count| {
        count.set(count.get() + 1);
        count.get()
    })
}

pub fn first() -> f64 {
    FIRST.with(|first| *first)
}
`;

function compile(name: string, source: string): { js: string; dir: string; code: number; stderr: string } {
  const dir = fixture(name);
  writeFileSync(join(dir, "lib.rs"), source);
  const out = runSync([compiler, join(dir, "lib.rs"), "-o", join(dir, "lib.js"), ...externs], dir, 120_000);
  return { js: out.code === 0 ? readFileSync(join(dir, "lib.js"), "utf8") : "", dir, code: out.code ?? -1, stderr: out.stderr };
}

type Global = { reenter?: () => number };

test("a thread-local whose initializer calls a binding that calls the crate back is made at its first use, in Rust's order", async () => {
  const { js, dir, code, stderr } = compile("calls-back-marked", program("#[cfg_attr(rust_js, rust_js::calls_back)]"));
  expect(code, stderr).toBe(0);
  // Made where it's first used, as `thread_local!` makes it.
  expect(js).toContain("const FIRST = $local(() => globalThis.reenter());");
  const calls: string[] = [];
  let lib: { bump(): number; first(): number } | undefined;
  (globalThis as Global).reenter = () => {
    calls.push(lib === undefined ? "reenter as the module loads" : "reenter");
    return lib === undefined ? -1 : lib.bump();
  };
  lib = await import(join(dir, "lib.js"));
  expect(calls).toEqual([]);
  // `bump` counts 1; `first` makes FIRST, whose `reenter` counts 2; made,
  // it isn't made again; `bump` counts 3.
  expect([lib!.bump(), lib!.first(), lib!.first(), lib!.bump()]).toEqual([1, 2, 2, 3]);
  expect(calls).toEqual(["reenter"]);
  delete (globalThis as Global).reenter;
});

test("unmarked, a binding is taken to call nothing back: its thread-local is made as its module loads", () => {
  const { js, code, stderr } = compile("calls-back-unmarked", program(""));
  expect(code, stderr).toBe(0);
  expect(js).not.toContain("$local(");
  expect(js).toContain("const FIRST = globalThis.reenter();");
});

test("calls_back beside a mark that says the binding runs none of the program's code is refused", () => {
  for (const [other, says] of [
    ["runs_nothing", "`#[rust_js::runs_nothing]` says it runs no code of the program's"],
    ["runs_only_given", "`#[rust_js::runs_only_given]` says it runs only what it's given"],
  ]) {
    const { code, stderr } = compile(
      `calls-back-and-${other}`,
      program(`#[cfg_attr(rust_js, rust_js::calls_back)]\n#[cfg_attr(rust_js, rust_js::${other})]`),
    );
    expect(code).not.toBe(0);
    expect(stderr).toContain(`\`#[rust_js::calls_back]\` can't be said of this (#2119): ${says}`);
  }
});

test("calls_back on what isn't a binding is refused", () => {
  const { code, stderr } = compile(
    "calls-back-not-a-binding",
    `#[cfg_attr(rust_js, rust_js::calls_back)]\npub fn own() -> u32 {\n    1\n}\n`,
  );
  expect(code).not.toBe(0);
  expect(stderr).toContain("it's on a binding, a function JS's code is");
});
