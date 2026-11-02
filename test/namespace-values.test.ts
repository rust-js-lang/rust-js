// A module's namespace that's a value (rd293, ADR 1061): a `pub`
// thread-local of a struct marked `#[rust_js::namespace]` is the value its
// module loads, whose fields are the module's exports, `export const {
// render, getRoot, act } = createReactNoop(..);`, as React's noop
// renderer's entries write it. Each test is a program of its own,
// compiled and run in Node.

import { expect, test } from "bun:test";
import { readFileSync, writeFileSync } from "node:fs";
import { join } from "node:path";
import { runSync } from "./child";
import { compiler, fixture, root, run } from "./support";

const noop = `use std::cell::Cell;

thread_local! {
    static MADE: Cell<u32> = const { Cell::new(0) };
}

/// What \`create\` gives.
pub struct Renderer {
    pub render: Box<dyn Fn(u32) -> u32>,
    pub get_root: fn() -> u32,
    #[cfg_attr(rust_js, rust_js::name = "_Scheduler")]
    pub scheduler: u32,
    /// None: an export that's \`undefined\`.
    pub act: Option<fn()>,
}

fn root() -> u32 {
    7
}

pub fn create(scale: u32) -> Renderer {
    MADE.set(MADE.get() + 1);
    Renderer {
        render: Box::new(move |n| n * scale),
        get_root: root,
        scheduler: scale + 1,
        act: None,
    }
}

pub fn made() -> u32 {
    MADE.get()
}
`;

/** A crate of the noop module, \`entry\`'s namespace of it, and \`lib\`. */
function compiled(name: string, entry: string, lib = "pub mod noop;\npub mod entry;\n") {
  const dir = fixture(name);
  writeFileSync(join(dir, "lib.rs"), lib);
  writeFileSync(join(dir, "noop.rs"), noop);
  writeFileSync(join(dir, "entry.rs"), entry);
  const p = runSync([compiler, "--declarations", join(dir, "lib.rs"), "-o", join(dir, "lib.js"), "--", "--crate-type", "lib"], root, 60_000);
  const read = (file: string) => readFileSync(join(dir, file), "utf8");
  return { p, dir, read };
}

const entry = `use crate::noop::{Renderer, create};

thread_local! {
    /// The renderer this module is.
    #[cfg_attr(rust_js, rust_js::namespace)]
    pub static NOOP: Renderer = create(3);
}
`;

test("a namespace thread-local's fields are its module's exports, made as it loads", async () => {
  const { p, dir, read } = compiled("namespace-value", entry);
  expect(p.stderr).not.toContain("error");
  expect(p.code).toBe(0);
  expect(read("entry.js")).toContain("export const { render, getRoot, _Scheduler, act } = create(3);\n");
  expect(read("entry.js")).not.toContain("NOOP");
  const types = read("entry.d.ts");
  expect(types).toContain("export const render: (value: number) => number;\n");
  expect(types).toContain("export const getRoot: () => number;\n");
  expect(types).toContain("export const _Scheduler: number;\n");
  expect(types).toContain("export const act: (() => void) | null | undefined;\n");
  const ran = run(["node", "--input-type=module", "--eval", `
    const entry = await import(${JSON.stringify(join(dir, "entry.js"))});
    const noop = await import(${JSON.stringify(join(dir, "noop.js"))});
    console.log(noop.made(), entry.render(2), entry.getRoot(), entry._Scheduler, entry.act === undefined, "act" in entry);
  `]).trim();
  // Made once, as the module loads; `act` an export that's `undefined`.
  expect(ran).toBe("1 6 7 4 true true");
});

test("a namespace thread-local Rust reads is refused, saying JS reads it", () => {
  const read = `${entry}
pub fn twice() -> u32 {
    NOOP.with(|noop| (noop.render)(2))
}
`;
  const { p } = compiled("namespace-value-read", read);
  expect(p.code).not.toBe(0);
  expect(p.stderr).toContain("rust-js: `NOOP` is its module's namespace, whose fields are the module's exports, which JS reads");
});

test("a namespace thread-local of what's no struct of named fields is refused", () => {
  const { p } = compiled(
    "namespace-value-unit",
    `thread_local! {
    #[cfg_attr(rust_js, rust_js::namespace)]
    pub static COUNT: u32 = 3;
}
`,
  );
  expect(p.code).not.toBe(0);
  expect(p.stderr).toContain("rust-js: a `#[rust_js::namespace]` thread-local is of a struct of named fields, its module's exports");
});

// An entry that exports its own list, as React's noop entries each write
// `export const {..} = createReactNoop(..)` of theirs (rd610, the user's
// decision, ADR 1037, amended): a struct of its own, a literal of which
// takes each field from the same-named field of one value it makes, is that
// value taken apart, in its struct's order.
const picked = `use crate::noop::{Renderer, create};

/// What this entry exports of the renderer.
pub struct Picked {
    /// The scheduler's export, its declaration's doc alone.
    #[cfg_attr(rust_js, rust_js::name = "_Scheduler")]
    pub scheduler: u32,
    // TODO: Remove this after callers migrate.
    pub render: Box<dyn Fn(u32) -> u32>,
    // $FlowFixMe[signature-verification-failure]
}

thread_local! {
    #[cfg_attr(rust_js, rust_js::namespace)]
    pub static NOOP: Picked = {
        let renderer = create(3);
        Picked {
            scheduler: renderer.scheduler,
            render: renderer.render,
        }
    };
}
`;

// Its struct's comments, above a field and after the last, are its export
// list's, as React's noop entries write theirs (rd610).
const list = (name: string) =>
  `export const {\n  ${name},\n  // TODO: Remove this after callers migrate.\n  render,\n  // $FlowFixMe[signature-verification-failure]\n} = `;

test("a namespace thread-local of an entry's own fields of one value is that value taken apart", async () => {
  const { p, dir, read } = compiled("namespace-value-picked", picked);
  expect(p.stderr).not.toContain("error");
  expect(p.code).toBe(0);
  const js = read("entry.js");
  expect(js).toContain(`${list("_Scheduler")}create(3);\n`);
  const types = read("entry.d.ts");
  expect(types).toContain("export const _Scheduler: number;\n");
  expect(types).not.toContain("getRoot");
  const ran = run(["node", "--input-type=module", "--eval", `
    const entry = await import(${JSON.stringify(join(dir, "entry.js"))});
    const noop = await import(${JSON.stringify(join(dir, "noop.js"))});
    console.log(noop.made(), entry.render(2), entry._Scheduler, Object.keys(entry).join());
  `]).trim();
  expect(ran).toBe("1 6 4 _Scheduler,render");
});

test("a namespace thread-local's literal of what isn't one value's same fields is made as written", async () => {
  const computed = picked.replace("scheduler: renderer.scheduler,", "scheduler: renderer.scheduler + 1,");
  const { p, dir, read } = compiled("namespace-value-computed", computed);
  expect(p.code).toBe(0);
  expect(read("entry.js")).toContain(`${list("_Scheduler")}(() => {\n  const renderer = create(3);\n`);
  const ran = run(["node", "--input-type=module", "--eval", `
    const entry = await import(${JSON.stringify(join(dir, "entry.js"))});
    console.log(entry.render(2), entry._Scheduler, Object.keys(entry).join());
  `]).trim();
  expect(ran).toBe("6 5 _Scheduler,render");
});

test("a namespace thread-local's literal of other names or another order is made as written", () => {
  // Its field another name in JS than what it's taken from.
  const renamed = picked.replace('    #[cfg_attr(rust_js, rust_js::name = "_Scheduler")]\n    pub scheduler', "    pub scheduler");
  const other = compiled("namespace-value-renamed", renamed);
  expect(other.p.code).toBe(0);
  expect(other.read("entry.js")).toContain(`${list("scheduler")}(() => {\n`);
  // Its literal in another order than its exports', which a getter reads in.
  const reordered = picked.replace(
    "            scheduler: renderer.scheduler,\n            render: renderer.render,\n",
    "            render: renderer.render,\n            scheduler: renderer.scheduler,\n",
  );
  const turned = compiled("namespace-value-reordered", reordered);
  expect(turned.p.code).toBe(0);
  expect(turned.read("entry.js")).toContain(`${list("_Scheduler")}(() => {\n`);
});
