// A method called where it's there, `js::optional_call` (#1859, ADR 1156):
// `value.method?.(args)`, the method read once and called with its value
// as its `this`, as upstream's composedClosest reads a hop's `closest`
// (vercel-labs/ui@220c1a0, packages/core/src/foundation/platform/platform.ts:64,
// `(hop as Element).closest?.(selector)`). Codex's regression: a getter
// read once, the receiver its `this`, an own `call` never read.

import { beforeAll, expect, test } from "bun:test";
import { readFileSync, writeFileSync } from "node:fs";
import { join } from "node:path";
import { buildReact, compiler, fixture, root, target } from "./support";

beforeAll(buildReact, 600_000);

const flags = ["--", "--extern", `webapi=${join(target, "libwebapi.rmeta")}`, "--extern", `js=${join(target, "libjs.rmeta")}`, "-L", target];

const program = `use webapi::Element;

pub fn closest_of(hop: &'static Element, selector: &str) -> Option<&'static Element> {
    js::optional_call((hop, Element::closest), (selector,)).flatten()
}

pub fn found(hop: &'static Element, selector: &str) -> u32 {
    match js::optional_call((hop, Element::closest), (selector,)) {
        None => 0,
        Some(None) => 1,
        Some(Some(_)) => 2,
    }
}

#[cfg_attr(rust_js, rust_js::link_name = "globalThis.log")]
fn log(what: &str) -> &'static Element {
    unreachable!()
}

pub fn ordered() -> Option<&'static Element> {
    js::optional_call((log("receiver"), Element::closest), (log("argument").tag_name().as_str(),)).flatten()
}
`;

test("an optional call is upstream's, its method read once of its this", async () => {
  const dir = fixture("js-optional-call");
  writeFileSync(join(dir, "lib.rs"), program);
  const built = Bun.spawnSync([compiler, "--declarations", join(dir, "lib.rs"), "-o", join(dir, "lib.js"), ...flags], { cwd: root, stderr: "pipe" });
  expect([built.exitCode, built.stderr.toString()]).toEqual([0, ""]);
  const js = readFileSync(join(dir, "lib.js"), "utf8");
  expect(js).toContain("export function closestOf(hop, selector) {\n  return hop.closest?.(selector);\n}");
  expect(js).toContain("$someDefined(hop.closest?.(selector))");
  // The receiver, then the argument, each run as Rust runs them.
  expect(js).toContain('  const log = globalThis.log("receiver");\n  const arg = globalThis.log("argument").tagName;\n  return log.closest?.(arg);');
  expect(readFileSync(join(dir, "lib.d.ts"), "utf8")).toContain("export function closestOf(hop: Element, selector: string): Element | null | undefined;");
  const lib = await import(join(dir, "lib.js"));
  // Codex's regression: one read of a getter, the receiver its `this`,
  // its own `call` never read.
  const reads: unknown[] = [];
  const receiver: Record<string, unknown> = {};
  function closest(this: unknown, selector: string) {
    reads.push(["invoke", this === receiver, selector]);
    return "found";
  }
  closest.call = () => { throw new Error("must not read/call .call"); };
  Object.defineProperty(receiver, "closest", { get() { reads.push("get"); return closest; } });
  expect(lib.closestOf(receiver, "button")).toBe("found");
  expect(reads).toEqual(["get", ["invoke", true, "button"]]);
  // A hop with no `closest`, one whose `closest` finds none, one that finds.
  expect([lib.found({}, "a"), lib.found({ closest: () => null }, "a"), lib.found({ closest: () => ({}) }, "a")]).toEqual([0, 1, 2]);
  expect([lib.closestOf({}, "a"), lib.closestOf({ closest: () => null }, "a")]).toEqual([undefined, null]);
  // A member that's there and no function is called: JS's TypeError.
  expect(() => lib.closestOf({ closest: 5 }, "a")).toThrow(TypeError);
  // In Rust's order: the receiver, then the argument, then the read.
  const order: string[] = [];
  (globalThis as Record<string, unknown>).log = (what: string) => {
    order.push(what);
    return { tagName: "B", get closest() { order.push("read"); return undefined; } };
  };
  expect(lib.ordered()).toBeUndefined();
  expect(order).toEqual(["receiver", "argument", "read"]);
});

// What `?.` can't say is refused, saying what to write: a getter, which
// is read, and a method that may give `undefined`, as `?.` gives of one
// that isn't there.
test("an optional call of a getter or of what may give undefined is refused, saying what to write", () => {
  const dir = fixture("js-optional-call-refused");
  for (const [body, refusal] of [
    ["pub fn tag(hop: &'static Element) -> Option<String> {\n    js::optional_call((hop, Element::tag_name), ())\n}", "is no method JS calls: read a getter as it is"],
    ["pub fn removed(hop: &'static Element) -> Option<()> {\n    js::optional_call((hop, Element::remove), ())\n}", "of a method that may give `undefined`"],
  ]) {
    writeFileSync(join(dir, "lib.rs"), `use webapi::Element;\n\n${body}\n`);
    const refused = Bun.spawnSync([compiler, join(dir, "lib.rs"), "-o", join(dir, "lib.js"), ...flags], { cwd: root, stderr: "pipe", timeout: 60_000 });
    expect([body, refused.exitCode === 0, refused.stderr.toString().includes(refusal)]).toEqual([body, false, true]);
  }
});
