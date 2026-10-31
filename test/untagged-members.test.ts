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

// Another untagged enum a variant holds is each of its kinds (rd305): one
// another variant's value may be, a container whose `children` tells it as
// an instance's does, or a number beside a number or a word, is refused,
// where it's dropped or compared, or where a `match` tells them apart.
test("an untagged enum in another is refused where its kinds are another variant's", () => {
  const dir = fixture("untagged-members-nested");
  writeFileSync(
    join(dir, "lib.rs"),
    `${children}
pub struct Container {
    #[cfg_attr(rust_js, rust_js::test)]
    pub children: RefCell<Vec<Child>>,
}

#[cfg_attr(rust_js, rust_js::untagged)]
pub enum Parent {
    Container(Rc<Container>),
    Child(Child),
}
`,
  );
  const dropped = runSync([compiler, ...flags(dir)], root, 60_000);
  expect(dropped.code).not.toBe(0);
  expect(dropped.stderr).toContain("`Child` and `Container` both hold an object whose `children` is an array or an object, which JS can't tell apart in an untagged enum (ADR 0214)");
  const source = `pub struct Leaf<'a> {
    #[cfg_attr(rust_js, rust_js::test)]
    pub children: &'a [Node<'a>],
}

pub struct Text {
    pub id: f64,
}

#[cfg_attr(rust_js, rust_js::untagged)]
pub enum Node<'a> {
    Leaf(&'a Leaf<'a>),
    Text(&'a Text),
}

pub struct Root<'a> {
    #[cfg_attr(rust_js, rust_js::test)]
    pub children: &'a [Node<'a>],
}

#[cfg_attr(rust_js, rust_js::untagged)]
pub enum Tree<'a> {
    Root(&'a Root<'a>),
    Node(Node<'a>),
}

pub fn is_root(tree: &Tree<'_>) -> bool {
    matches!(tree, Tree::Root(_))
}

#[cfg_attr(rust_js, rust_js::untagged)]
pub enum Key {
    Index(f64),
    Name(&'static str),
}

#[cfg_attr(rust_js, rust_js::untagged)]
pub enum Keyed {
    One(Key),
    Count(u32),
}

pub fn is_count(keyed: &Keyed) -> bool {
    matches!(keyed, Keyed::Count(_))
}
`;
  // One that holds itself, or another that holds a third, isn't told so.
  writeFileSync(
    join(dir, "lib.rs"),
    `#[cfg_attr(rust_js, rust_js::untagged)]
pub enum Deep {
    Inner(Box<Deep>),
    Leaf(f64),
}
`,
  );
  const deep = runSync([compiler, ...flags(dir)], root, 60_000);
  expect(deep.code).not.toBe(0);
  expect(deep.stderr).toContain("JS can't tell a `std::boxed::Box<Deep>` from another variant's value");
  writeFileSync(join(dir, "lib.rs"), source);
  const ran = runSync([compiler, ...flags(dir)], root, 60_000);
  expect(ran.code).not.toBe(0);
  expect(ran.stderr).toContain("`Root` and `Node` hold values of one kind, which JS can't tell apart in an untagged enum (ADR 0214)");
  expect(ran.stderr).toContain("`Count` and `One` hold values of one kind, which JS can't tell apart in an untagged enum (ADR 0214)");
});
