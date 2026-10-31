// An untagged enum of structs told apart by a member each declares,
// `#[rust_js::test]` on a field (rd291): React's noop renderer's
// `Instance | TextInstance`, `isArray(child.children)`. Each test is a
// program of its own, compiled and run.

import { expect, test } from "bun:test";
import { readFileSync, writeFileSync } from "node:fs";
import { join } from "node:path";
import { runSync } from "./child";
import { compiler, fixture, root, run, target } from "./support";

const flags = (dir: string) => [join(dir, "lib.rs"), "-o", join(dir, "lib.js"), "--", "--extern", `js=${join(target, "libjs.rmeta")}`, "-L", target];

const children = `use std::cell::RefCell;
use std::rc::Rc;

pub struct Instance {
    #[cfg_attr(rust_js, rust_js::test)]
    pub children: RefCell<Vec<Child>>,
    pub id: f64,
}

pub struct TextInstance {
    pub text: String,
    pub id: f64,
}

#[cfg_attr(rust_js, rust_js::untagged)]
pub enum Child {
    Instance(Rc<Instance>),
    Text(Rc<TextInstance>),
}
`;

test("an untagged enum's struct is told apart by the member it declares", async () => {
  const dir = fixture("untagged-members");
  writeFileSync(
    join(dir, "lib.rs"),
    `${children}
pub fn is_text(child: &Child) -> bool {
    matches!(child, Child::Text(_))
}

pub fn id(child: &Child) -> f64 {
    match child {
        Child::Instance(instance) => instance.id,
        Child::Text(text) => text.id,
    }
}
`,
  );
  run([compiler, ...flags(dir)]);
  const js = readFileSync(join(dir, "lib.js"), "utf8");
  // Each variant an object, the one told by its member alone.
  expect(js).toContain("return !Array.isArray(child.children);");
  expect(js).toContain("return child.id;");
  const lib = await import(join(dir, "lib.js"));
  expect([lib.isText({ children: [], id: 1 }), lib.isText({ text: "a", id: 2 }), lib.id({ text: "a", id: 2 })]).toEqual([false, true, 2]);
});

test("a member that tells a struct apart is of a kind JS tells", () => {
  const dir = fixture("untagged-members-object");
  writeFileSync(
    join(dir, "lib.rs"),
    children.replace("pub children: RefCell<Vec<Child>>,", "pub children: TextInstance,").replace("#[cfg_attr(rust_js, rust_js::untagged)]", "#[cfg_attr(rust_js, rust_js::untagged)]\n#[derive(PartialEq)]") +
      "impl PartialEq for TextInstance {\n    fn eq(&self, other: &Self) -> bool {\n        self.id == other.id\n    }\n}\nimpl PartialEq for Instance {\n    fn eq(&self, other: &Self) -> bool {\n        self.id == other.id\n    }\n}\n",
  );
  const ran = runSync([compiler, ...flags(dir)], root, 60_000);
  expect(ran.code).not.toBe(0);
  expect(ran.stderr).toContain("is told apart by its `children`, `#[rust_js::test]`, which holds a string, a number, a `bool`, an array or a class");
});
