// A live accessor of a struct a literal makes, `js::Accessor<T>` (ADR 1164,
// B): a field of one is an own, enumerable, configurable getter without a
// setter, `get canceled() { return canceled; }`, whose read is what its
// function gives then, as upstream's createDetails writes `canceled`
// (vercel-labs/ui@220c1a0; Codex's contract, vercel-ui-sdk-in-rust@2680f613e).
// `details.canceled.get()` is `details.canceled`; a spread of the details
// has the value it gives then. A read of it another way, which JS would
// read otherwise, is refused.

import { beforeAll, expect, test } from "bun:test";
import { readFileSync, writeFileSync } from "node:fs";
import { join } from "node:path";
import { buildReact, compiler, fixture, root, run, target } from "./support";

beforeAll(buildReact, 600_000);

const withJs = ["--", "--extern", `js=${join(target, "libjs.rmeta")}`, "-L", target];

const program = `use std::cell::Cell;
use std::rc::Rc;

/// A change's details.
pub struct Details {
    /// Stops the change.
    pub cancel: Box<dyn Fn()>,
    /// Whether \`cancel()\` was called.
    pub canceled: js::Accessor<bool>,
}

pub fn details() -> Details {
    let canceled = Rc::new(Cell::new(false));
    let set = Rc::clone(&canceled);
    Details {
        cancel: Box::new(move || set.set(true)),
        canceled: js::Accessor::new(move || canceled.get()),
    }
}

pub fn read(details: &Details) -> bool {
    details.canceled.get()
}
`;

test("an accessor field is a live own getter, read by its get", () => {
  const dir = fixture("js-accessor");
  writeFileSync(join(dir, "lib.rs"), program);
  run([compiler, "--declarations", join(dir, "lib.rs"), "-o", join(dir, "lib.js"), ...withJs]);
  const js = readFileSync(join(dir, "lib.js"), "utf8");
  expect(js).toContain("    get canceled() {\n      return canceled;\n    },\n");
  expect(js).toContain("export function read(details) {\n  return details.canceled;\n}");
  expect(readFileSync(join(dir, "lib.d.ts"), "utf8")).toContain("    readonly canceled: boolean;");
  writeFileSync(join(dir, "run.mjs"), `import { details, read } from "./lib.js";
const d = details();
const descriptor = Object.getOwnPropertyDescriptor(d, "canceled");
const before = { ...d };
const { cancel } = d;
const other = details();
cancel();
cancel();
console.log(JSON.stringify([typeof descriptor.get, descriptor.set, descriptor.enumerable, descriptor.configurable, before.canceled, d.canceled, read(d), other.canceled]));
`);
  // No setter (`null` of JSON's `undefined`); the spread before `cancel()`
  // kept `false`; another's its own.
  expect(run(["node", join(dir, "run.mjs")])).toBe('["function",null,true,true,false,true,true,false]\n');
});

test("an accessor read but by its get of a struct's field is refused", () => {
  const dir = fixture("js-accessor-refused");
  writeFileSync(join(dir, "lib.rs"), `pub struct Details {
    pub canceled: js::Accessor<bool>,
}

pub fn local() -> bool {
    let acc = js::Accessor::new(|| true);
    acc.get()
}

pub fn moved(details: Details) -> bool {
    let Details { canceled } = details;
    canceled.get()
}
`);
  const refused = Bun.spawnSync([compiler, join(dir, "lib.rs"), "-o", join(dir, "lib.js"), "--", "--crate-type=lib", "--extern", `js=${join(target, "libjs.rmeta")}`, "-L", target], { cwd: root, stderr: "pipe" });
  const said = refused.stderr.toString();
  expect([refused.exitCode === 0, said.includes("`js::Accessor::get` reads a struct's field"), said.includes("a `js::Accessor`'s field is read only by its `get`")]).toEqual([false, true, true]);
});
