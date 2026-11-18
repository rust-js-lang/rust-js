// A flattened `js::Spread<&G>` field, the copy of a value's own enumerable
// keys a struct's object begins with (ADR 1164, G): `{ ...given, source }`,
// as upstream's createDetails writes its details (vercel-labs/ui@220c1a0;
// Codex's contract, vercel-ui-sdk-in-rust@2680f613e). JS's own spread, so
// a getter and a Proxy's traps run as upstream's literal runs them, symbols
// are copied, an own `undefined` stays a key, what isn't own or enumerable
// isn't, the struct's own keys after it win, and the value itself is never
// written. Declared `{ source: string } & Omit<G, "source">`.

import { beforeAll, expect, test } from "bun:test";
import { readFileSync, writeFileSync } from "node:fs";
import { join } from "node:path";
import { buildReact, compiler, fixture, root, run, target } from "./support";
import { es2022, type Setting, typeChecked } from "./type-checked";

beforeAll(buildReact, 600_000);

const withJs = ["--", "--extern", `js=${join(target, "libjs.rmeta")}`, "-L", target];

// What this file's checks of its declarations are made under (`typeChecked`).
const checking: Setting = { ...es2022, lib: ["es2022"], types: [] };

const program = `/// A commit's details: what it's given, then how it came about.
pub struct Committed<G: 'static> {
    /// What it's given, its own keys first.
    #[cfg_attr(rust_js, rust_js::flatten)]
    pub given: js::Spread<&'static G>,
    /// How it came about.
    pub source: &'static str,
}

pub fn committed<G>(given: &'static G, source: &'static str) -> Committed<G> {
    Committed { given: js::Spread::new(given), source }
}
`;

test("a flattened js::Spread is JS's own spread of the value, then the struct's keys", () => {
  const dir = fixture("js-spread");
  writeFileSync(join(dir, "lib.rs"), program);
  run([compiler, "--declarations", join(dir, "lib.rs"), "-o", join(dir, "lib.js"), ...withJs]);
  expect(readFileSync(join(dir, "lib.js"), "utf8")).toContain("  return {\n    ...given,\n    /** How it came about. */\n    source,\n  };");
  expect(readFileSync(join(dir, "lib.d.ts"), "utf8")).toContain(`export type Committed<G> = {
    /** How it came about. */
    source: string;
} & Omit<G, "source">;`);
  // Of the same Proxy and getter, the trace ours makes is upstream's literal's.
  writeFileSync(join(dir, "run.mjs"), `import { committed } from "./lib.js";
const tag = Symbol("tag");
const traced = (log) => {
  const target = { reason: "escape-key", event: undefined, source: "theirs", [tag]: 1 };
  Object.defineProperty(target, "hidden", { value: 1, enumerable: false });
  Object.defineProperty(target, "read", { get() { log.push("get read"); return 2; }, enumerable: true });
  return new Proxy(target, {
    ownKeys(t) { log.push("ownKeys"); return Reflect.ownKeys(t); },
    getOwnPropertyDescriptor(t, k) { log.push("descriptor " + String(k)); return Reflect.getOwnPropertyDescriptor(t, k); },
    get(t, k, r) { log.push("get " + String(k)); return Reflect.get(t, k, r); },
    set() { log.push("set"); return false; },
  });
};
const ours = [], theirs = [];
const made = committed(traced(ours), "mouse");
const reference = { ...traced(theirs), source: "mouse" };
console.log(JSON.stringify([ours, ours.join() === theirs.join(), Object.keys(made), "event" in made, made.source, made[tag], "hidden" in made, made.read]));
`);
  const [trace, same, keys, event, source, tagged, hidden, read] = JSON.parse(run(["node", join(dir, "run.mjs")]));
  expect([same, keys, event, source, tagged, hidden, read]).toEqual([true, ["reason", "event", "source", "read"], true, "mouse", 1, false, 2]);
  expect(trace).toContain("ownKeys");
  expect(trace).not.toContain("set");
  writeFileSync(join(dir, "consumer.ts"), `import { committed } from "./lib.js";
const given = { reason: "escape-key", extra: 1 } as const;
export const extra: 1 = committed(given, "mouse").extra;
export const source: string = committed(given, "mouse").source;
// @ts-expect-error The struct's own key is its own, not the given's.
export const theirs: "theirs" = committed({ source: "theirs" } as const, "mouse").source;
`);
  expect(typeChecked(dir, "consumer.ts", checking)).toEqual([]);
});

test("a js::Spread unflattened, or alone in its struct, is refused", () => {
  const dir = fixture("js-spread-refused");
  writeFileSync(join(dir, "lib.rs"), `pub struct Unflattened<G: 'static> {
    pub given: js::Spread<&'static G>,
    pub source: &'static str,
}

pub struct Alone<G: 'static> {
    #[cfg_attr(rust_js, rust_js::flatten)]
    pub given: js::Spread<&'static G>,
}
`);
  const refused = Bun.spawnSync([compiler, join(dir, "lib.rs"), "-o", join(dir, "lib.js"), "--", "--crate-type=lib", "--extern", `js=${join(target, "libjs.rmeta")}`, "-L", target], { cwd: root, stderr: "pipe" });
  const said = refused.stderr.toString();
  expect([refused.exitCode === 0, said.includes("a `js::Spread` field is flattened"), said.includes("is the copy itself, `js::object::spread(value)`")]).toEqual([false, true, true]);
});
