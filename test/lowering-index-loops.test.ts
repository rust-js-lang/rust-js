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
  const p = Bun.spawnSync(["bun", "test", "--preload", "./test/happydom.js", join(dir, "driver.test.js")], { cwd: root, stderr: "pipe" });
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

// React DOM's Fizz instruction set's `suspenseyImages.push(...)` of each
// image a `NodeList` holds: a `Vec`'s `extend` of an indexed list, a
// `NodeList`'s, an array's or a slice's, is an index loop that pushes each
// item, as many as the list has. Not `push.apply(v, list)` or
// `push(...list)`, which throw a `RangeError` past the engine's limit of
// arguments, where Rust's `extend` never fails; and no `Iterator.from` an
// ES5 build polyfills (the user's decision, 2026-11-01).
test("a Vec's extend of an indexed list pushes each in an index loop", () => {
  const dir = fixture("indexed-extend");
  writeFileSync(join(dir, "lib.rs"), `use webapi::{Element, NodeList, node_list};

pub fn gathered(list: &NodeList<Element>) -> Vec<String> {
    let mut images: Vec<&Element> = Vec::new();
    // Each image, in order.
    images.extend(node_list::iter(list));
    images.iter().map(|e| e.tag_name()).collect()
}

pub fn appended(xs: &[u32], more: Vec<u32>) -> Vec<u32> {
    let mut out = vec![0];
    out.extend(xs);
    out.extend(xs.iter());
    out.extend(more);
    out
}

pub fn huge(n: usize) -> usize {
    let items = vec![1u8; n];
    let mut out: Vec<u8> = Vec::new();
    out.extend(items.iter());
    out.len()
}
`);
  run([compiler, join(dir, "lib.rs"), "-o", join(dir, "lib.js"), "--", "--extern", `webapi=${join(target, "libwebapi.rmeta")}`, "--extern", `js=${join(target, "libjs.rmeta")}`, "-L", target]);
  const js = readFileSync(join(dir, "lib.js"), "utf8");
  expect(js).toContain("  // Each image, in order.\n  for (let i = 0; i < list.length; i++) {\n    images.push(list[i]);\n  }\n");
  // Of a slice's iterator made here, the slice's; of a slice or a `Vec`
  // given whole, as the call's own type for what it's given says, the
  // same (a trait's dictionary, given any type, keeps `$extend`).
  expect(js).toContain(
    "  for (let i = 0; i < xs.length; i++) {\n    out.push(xs[i]);\n  }\n  for (let i = 0; i < xs.length; i++) {\n    out.push(xs[i]);\n  }\n  for (let i = 0; i < more.length; i++) {\n    out.push(more[i]);\n  }\n",
  );
  expect(js).toContain("  for (let i = 0; i < items.length; i++) {\n    out.push(items[i]);\n  }\n");
  writeFileSync(join(dir, "driver.test.js"), `import { expect, test } from "bun:test";
import { appended, gathered, huge } from "./lib.js";

test("as Rust's extend", () => {
  const div = document.createElement("div");
  div.innerHTML = "<img><b></b><img>";
  expect(gathered(div.childNodes)).toEqual(["IMG", "B", "IMG"]);
  expect(appended([1, 2], [3])).toEqual([0, 1, 2, 1, 2, 3]);
  // Past any engine's limit of a call's arguments.
  expect(huge(300000)).toBe(300000);
});
`);
  const p = Bun.spawnSync(["bun", "test", "--preload", "./test/happydom.js", join(dir, "driver.test.js")], { cwd: root, stderr: "pipe" });
  const output = p.stdout.toString() + p.stderr.toString();
  expect([p.exitCode, output.match(/(\d+) pass/)?.[1]], output).toEqual([0, "1"]);
});

// React DOM's setInitialProperties: a comment above a thread-local's
// `with`, whose closure is the loop, is above the `for`, as upstream's
// `// We listen to these events ..` is, not inside its test before the
// array's `length` (rd355): the array the test and each item read is
// where the loop is, not where the `with` was written.
test("a comment above a loop over a thread-local's array is above the for", async () => {
  const dir = fixture("indexed-comment");
  writeFileSync(join(dir, "lib.rs"), `thread_local! {
    static MEDIA_EVENT_TYPES: Vec<&'static str> = vec!["abort", "canplay"];
}

fn listen(name: &str, out: &mut Vec<String>) {
    out.push(name.to_string());
}

pub fn set_initial(tag: &str) -> Vec<String> {
    let mut out = Vec::new();
    match tag {
        "video" | "audio" => {
            // We listen to these events in case to ensure emulated bubble
            // listeners still fire for all the media events.
            MEDIA_EVENT_TYPES.with(|media_event_types| {
                for media_event_type in media_event_types {
                    listen(media_event_type, &mut out);
                }
            });
        }
        _ => {}
    }
    out
}
`);
  run([compiler, join(dir, "lib.rs"), "-o", join(dir, "lib.js")]);
  const js = readFileSync(join(dir, "lib.js"), "utf8");
  expect(js).toContain(
    "      // We listen to these events in case to ensure emulated bubble\n      // listeners still fire for all the media events.\n      for (let i = 0; i < mediaEventTypes.length; i++) {\n        const mediaEventType = mediaEventTypes[i];\n",
  );
  const lib = await import(join(dir, "lib.js"));
  expect([lib.setInitial("video"), lib.setInitial("p")]).toEqual([["abort", "canplay"], []]);
});

// React DOM's retryIfBlockedOn: an index loop in a thread-local's `with`,
// `for (let i = 0; ..)`, then the program's own `let mut i = 0` in an `if`:
// each its own `let i`, as upstream writes them, the loop's block-scoped,
// not the program's declared above the loop, `let i;`, and assigned `i =
// 0;` where it's written (rd356).
test("an index loop's counter is no program's later let of its name", async () => {
  const dir = fixture("indexed-counter-name");
  writeFileSync(join(dir, "lib.rs"), `use std::cell::{Cell, RefCell};
use std::rc::Rc;

pub struct Target {
    pub blocked: Cell<u32>,
}

thread_local! {
    static TARGETS: RefCell<Vec<Rc<Target>>> = const { RefCell::new(Vec::new()) };
}

pub fn add(blocked: u32) {
    TARGETS.with_borrow_mut(|targets| targets.push(Rc::new(Target { blocked: Cell::new(blocked) })));
}

pub fn retry(unblocked: u32, queue: Option<&[f64]>) -> f64 {
    TARGETS.with_borrow(|targets| {
        for target in targets {
            if target.blocked.get() == unblocked {
                target.blocked.set(0);
            }
        }
    });
    let mut n = 0.0;
    if let Some(queue) = queue {
        let mut i = 0.0;
        while i < queue.len() as f64 {
            let x = queue[i as usize];
            if x > 10.0 {
                i += 3.0;
                continue;
            }
            n += x;
            i += 1.0;
        }
    }
    n
}

pub fn blocked() -> Vec<u32> {
    TARGETS.with_borrow(|targets| targets.iter().map(|t| t.blocked.get()).collect())
}
`);
  run([compiler, join(dir, "lib.rs"), "-o", join(dir, "lib.js")]);
  const js = readFileSync(join(dir, "lib.js"), "utf8");
  expect(js).toContain("export function retry(unblocked, queue) {\n  for (let i = 0; i < targets.length; i++) {\n");
  expect(js).toContain("  if (queue != null) {\n    let i = 0;\n    while (i < queue.length) {\n");
  expect(js).not.toContain("let i;");
  const lib = await import(join(dir, "lib.js"));
  lib.add(1);
  lib.add(2);
  lib.add(1);
  // As native Rust prints them: `16 0 [0, 2, 0]`.
  expect([lib.retry(1, [1, 2, 11, 4, 5, 6, 7]), lib.retry(9, undefined), lib.blocked()]).toEqual([16, 0, [0, 2, 0]]);
});

// React's Fizz class components' processUpdateQueue, `for (let i =
// oldReplace ? 1 : 0; i < oldQueue.length; i++)`: a loop over a range of a
// slice the body's ranges show in bounds, `&v[1..]`, `&v[..2]`, `&v[1..2]`,
// counts its indices from the range's start to its end, `v.length` or the
// end, no copy, `v.slice(1)`, made (rd376); siblings each `i`. One not
// shown in bounds stays `$slice(v, start)`, which panics past the end, as
// Rust's range does.
test("a loop over a range of a slice in bounds counts its indices", async () => {
  const dir = fixture("indexed-ranges");
  writeFileSync(join(dir, "lib.rs"), `pub fn known(v: &[u32]) -> u32 {
    let mut n = 0;
    if v.len() > 2 {
        for x in &v[1..] {
            n += x;
        }
        for x in &v[..2] {
            n += 10 * x;
        }
        for x in &v[1..2] {
            n += 100 * x;
        }
    }
    n
}

pub fn tail(v: &[u32], start: usize) -> u32 {
    let mut n = 0;
    for &x in &v[start..] {
        n += x;
    }
    n
}

pub fn shrinking(v: &[u32]) -> u32 {
    let mut n = 0;
    if v.len() > 2 {
        let mut end = 2;
        for x in &v[..end] {
            end -= 1;
            n += x * 10 + end as u32;
        }
    }
    n
}
`);
  run([compiler, join(dir, "lib.rs"), "-o", join(dir, "lib.js")]);
  const js = readFileSync(join(dir, "lib.js"), "utf8");
  expect(js).toContain("    for (let i = 1; i < v.length; i++) {\n      const x = v[i];\n");
  expect(js).toContain("    for (let i = 0; i < 2; i++) {\n      const x = v[i];\n");
  expect(js).toContain("    for (let i = 1; i < 2; i++) {\n      const x = v[i];\n");
  expect(js).toContain("  for (const x of $slice(v, start)) {\n");
  // An end the body changes, which Rust read once: the range's copy.
  expect(js).toContain("    for (const x of v.slice(0, end)) {\n");
  const lib = await import(join(dir, "lib.js"));
  // As native Rust prints them: `235 0 5 0 31`, then a panic past the end.
  expect([lib.known([1, 2, 3]), lib.known([1, 2]), lib.tail([1, 2, 3], 1), lib.tail([1, 2, 3], 3), lib.shrinking([1, 2, 3])]).toEqual([235, 0, 5, 0, 31]);
  expect(() => lib.tail([1, 2, 3], 5)).toThrow("range start index 5 out of range for slice of length 3");
});

// React's areHookInputsEqual: an index tested below two lengths, `i <
// prev.len() && i < next.len()`, reads each slice as JS does, `prev[i]` and
// `next[i]`, in a closure too, as Rust's borrows keep what it captures as
// it was where it's made (rd423).
test("an index below two lengths reads each in bounds, in a closure too", async () => {
  const dir = fixture("index-below-two-lengths");
  writeFileSync(join(dir, "lib.rs"), `use std::cell::Cell;

thread_local! {
    static CALLS: Cell<u32> = const { Cell::new(0) };
}

fn same(a: &u32, b: &u32) -> bool {
    CALLS.set(CALLS.get() + 1);
    a == b
}

pub fn equal(next: &[u32], prev: &[u32]) -> bool {
    let mut i = 0;
    while i < prev.len() && i < next.len() {
        if CALLS.with(|_| same(&next[i], &prev[i])) {
            i += 1;
            continue;
        }
        return false;
    }
    true
}
`);
  run([compiler, join(dir, "lib.rs"), "-o", join(dir, "lib.js"), "--", "--extern", `js=${join(target, "libjs.rmeta")}`, "-L", target]);
  const js = readFileSync(join(dir, "lib.js"), "utf8");
  expect(js).toContain("same(next[i], prev[i])");
  expect(js).not.toContain("$index");
  const lib = await import(join(dir, "lib.js"));
  expect(lib.equal([1, 2, 3], [1, 2, 3])).toBe(true);
  expect(lib.equal([1, 2], [1, 2, 3])).toBe(true);
  expect(lib.equal([4, 2], [1, 2, 3])).toBe(false);
});
