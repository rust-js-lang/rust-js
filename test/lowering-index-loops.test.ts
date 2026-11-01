// What the compiler writes of a loop over an indexed list's items, an
// array's or a binding's `NodeList`'s: an index loop, `for (let i = 0; i <
// v.length; i++) { const x = v[i]; .. }`, as upstream writes it (ADR 1042).
// Each test is a program of its own, compiled and run.

import { beforeAll, expect, test } from "bun:test";
import { readFileSync, writeFileSync } from "node:fs";
import { join } from "node:path";
import { runSync } from "./child";
import { buildReact, compiler, fixture, root, run, target } from "./support";

beforeAll(buildReact, 600_000);

// A `for` whose binding is `mut`, which MIR declares before the loop and
// assigns each turn: a `for…of` all the same, of an iterator stepped
// first too.
test("a for loop of a mut binding is a for...of", async () => {
  const dir = fixture("for-mut-binding");
  writeFileSync(join(dir, "lib.rs"), `pub fn plain_mut(v: Vec<Vec<u32>>) -> usize {
    let mut n = 0;
    for mut row in v {
        row.push(0);
        n += row.len();
    }
    n
}
pub fn rest_mut(v: Vec<Vec<u32>>) -> usize {
    let mut it = v.into_iter();
    it.next();
    let mut n = 0;
    for mut row in it {
        row.push(0);
        n += row.len();
    }
    n
}
`);
  run([compiler, join(dir, "lib.rs"), "-o", join(dir, "lib.js")]);
  const js = readFileSync(join(dir, "lib.js"), "utf8");
  expect(js).toContain("  for (let i = 0; i < v.length; i++) {\n    const row = v[i];\n    row.push(0);");
  expect(js).toContain("  for (const row of it) {\n    row.push(0);");
  expect(js).not.toContain("let row;");
  const lib = await import(join(dir, "lib.js"));
  expect([lib.plainMut([[1], [2, 3]]), lib.restMut([[1], [2, 3], [4]])]).toEqual([5, 5]);
});

// A `NodeList`'s, an `HTMLCollection`'s: WebIDL makes an indexed list's
// `[Symbol.iterator]` `Array.prototype.values`, which reads `length` and
// then `list[i]` at each step, so an index loop steps what `for…of` does,
// a live list's items too, as the body takes them out of it: React DOM's
// Fizz instruction set's `for (let j = 0; j < appearingElements.length;
// j++)`. A list read anew each turn, of no variable, stays a `for…of`.
test("a for over a NodeList's items is an index loop, live as for…of", () => {
  const dir = fixture("indexed-lists");
  writeFileSync(join(dir, "lib.rs"), `use webapi::{Element, HTMLCollection, NodeList, html_collection, node_list};

pub fn tags(list: &NodeList<Element>) -> Vec<String> {
    let mut out = Vec::new();
    for element in node_list::iter(list) {
        out.push(element.tag_name());
    }
    out
}

pub fn removed(list: &HTMLCollection) -> Vec<String> {
    let mut out = Vec::new();
    for element in html_collection::iter(list) {
        out.push(element.tag_name());
        element.remove();
    }
    out
}

pub fn found(root: &Element) -> usize {
    let mut n = 0;
    for _ in node_list::iter(root.query_selector_all("b")) {
        n += 1;
    }
    n
}
`);
  run([compiler, join(dir, "lib.rs"), "-o", join(dir, "lib.js"), "--", "--extern", `webapi=${join(target, "libwebapi.rmeta")}`, "--extern", `js=${join(target, "libjs.rmeta")}`, "-L", target]);
  const js = readFileSync(join(dir, "lib.js"), "utf8");
  expect(js).toContain("  for (let i = 0; i < list.length; i++) {\n    const element = list[i];\n    out.push(element.tagName);\n");
  expect(js).toContain("  for (let i = 0; i < list.length; i++) {\n    const element = list[i];\n    out.push(element.tagName);\n    element.remove();\n");
  expect(js).toContain('  for (const _ of root.querySelectorAll("b")) {\n');
  // Run in a DOM, against `for…of` of the same lists: happy-dom's, and one
  // as WebIDL has it, whose items a getter reads live.
  writeFileSync(join(dir, "driver.test.js"), `import { expect, test } from "bun:test";
import { found, removed, tags } from "./lib.js";

const made = () => {
  const div = document.createElement("div");
  div.innerHTML = "<a></a><b></b><i></i><b></b><u></u>";
  return div;
};
const ofEach = (list, f) => {
  const out = [];
  for (const element of list) out.push(f(element));
  return out;
};
/** A live list as WebIDL makes one: \`length\` and each index a getter of
 * \`items\`, its iterator \`Array.prototype.values\`. */
const live = (items) => {
  const element = (tagName) => ({ tagName, remove() { items.splice(items.indexOf(this), 1); } });
  items.splice(0, items.length, ...items.map(element));
  return new Proxy({ [Symbol.iterator]: Array.prototype.values }, {
    get(target, key) {
      if (key === "length") return items.length;
      if (typeof key === "string" && /^\\d+$/.test(key)) return items[Number(key)];
      return target[key];
    },
  });
};

test("as for…of steps them", () => {
  expect(tags(made().childNodes)).toEqual(ofEach(made().childNodes, (e) => e.tagName));
  expect(removed(made().children)).toEqual(ofEach(made().children, (e) => (e.remove(), e.tagName)));
  expect(removed(made().children)).toEqual(["A", "I", "U"]);
  expect(removed(live(["A", "B", "I"]))).toEqual(ofEach(live(["A", "B", "I"]), (e) => (e.remove(), e.tagName)));
  expect(found(made())).toBe(2);
});
`);
  const p = Bun.spawnSync(["bun", "test", "--preload", "./test/happydom.ts", join(dir, "driver.test.js")], { cwd: root, stderr: "pipe" });
  const output = p.stdout.toString() + p.stderr.toString();
  expect([p.exitCode, output.match(/(\d+) pass/)?.[1]], output).toEqual([0, "1"]);
});

// The fact is of a binding's struct, the list's type: of anything else it
// says nothing, and is refused.
test("#[rust_js::indexed] of anything but a struct is refused", () => {
  const dir = fixture("indexed-refused");
  writeFileSync(join(dir, "lib.rs"), "#[cfg_attr(rust_js, rust_js::indexed)]\npub fn list() {}\n");
  const p = runSync([compiler, join(dir, "lib.rs"), "-o", join(dir, "lib.js")], root, 60_000);
  expect(p.code).not.toBe(0);
  expect(p.stderr).toContain("`#[rust_js::indexed]` is of a binding's struct");
});
