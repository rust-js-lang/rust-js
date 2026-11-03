// A flattened enum told by its keys (#504, ADR 1071), as alien-signals'
// nodes are one object each, told apart by `'getter' in node`: what its
// declarations say, and what's refused. The corpus's
// `flattened_keyed_enum` and `flattened_keyed_param` run it against
// native Rust.

import { expect, test } from "bun:test";
import { readFileSync, writeFileSync } from "node:fs";
import { join } from "node:path";
import { runSync } from "./child";
import { compiler, fixture, root } from "./support";

/** What the compiler says of `rust`, which it refuses. */
function refused(name: string, rust: string): string {
  const dir = fixture(name);
  writeFileSync(join(dir, "lib.rs"), rust);
  const ran = runSync([compiler, join(dir, "lib.rs"), "-o", join(dir, "lib.js")], root, 600_000);
  expect(ran.code).not.toBe(0);
  return ran.stderr;
}

const kind = `pub enum Kind {
    Signal { current: Option<u32> },
    Computed {
        value: Option<u32>,
        #[cfg_attr(rust_js, rust_js::test)]
        getter: fn() -> u32,
    },
    Scope,
}
`;

// Each variant an object of its keys alone, no tag, `{}` of the one
// without fields; a struct a type parameter or a keyed enum is flattened
// in, what it is besides, `{ .. } & K`, as an interface can extend
// neither: upstream's interfaces that extend `ReactiveNode`, together.
test("a keyed enum is declared its variants' keys, and what flattens it an intersection", () => {
  const dir = fixture("flattened-keyed-declarations");
  writeFileSync(
    join(dir, "lib.rs"),
    `${kind}
pub struct Node<K> {
    pub flags: u32,
    #[cfg_attr(rust_js, rust_js::flatten)]
    pub kind: K,
}

pub struct Own {
    pub id: u32,
    #[cfg_attr(rust_js, rust_js::flatten)]
    pub kind: Kind,
}

pub fn scope() -> Node<Kind> {
    Node { flags: 1, kind: Kind::Scope }
}

pub fn own() -> Own {
    Own { id: 2, kind: Kind::Signal { current: None } }
}

pub fn is_computed(own: &Own) -> bool {
    matches!(own.kind, Kind::Computed { .. })
}

pub enum Mode {
    Idle,
    Busy { since: u32 },
}

pub fn idle() -> Node<Mode> {
    Node { flags: 0, kind: Mode::Idle }
}
`,
  );
  const ran = runSync([compiler, "--declarations", join(dir, "lib.rs"), "-o", join(dir, "lib.js")], root, 600_000);
  expect(ran.code, ran.stderr).toBe(0);
  const declarations = readFileSync(join(dir, "lib.d.ts"), "utf8");
  expect(declarations).toContain(`export type Kind = {
    current: number | null | undefined;
} | {
    value: number | null | undefined;
    getter: () => number;
} | {};`);
  expect(declarations).toContain(`export type Node<K> = {
    flags: number;
} & K;`);
  expect(declarations).toContain(`export type Own = {
    id: number;
} & Kind;`);
  expect(declarations).toContain(`export type Mode = {} | {
    since: number;
};`);
  expect(declarations).not.toContain("TAG");
  const js = readFileSync(join(dir, "lib.js"), "utf8");
  expect(js).toContain("return { flags: 1 };");
  expect(js).toContain("return { id: 2, current: undefined };");
  expect(js).toContain('return "getter" in own;');
  expect(js).not.toContain("TAG");
});

test("a flattened enum no key tells, a key its parent has, and a type parameter given another type are refused", () => {
  // Two variants without fields, which no key tells apart.
  expect(
    refused(
      "flattened-keyed-two-units",
      `pub enum Kind { Signal { current: u32 }, Scope, Root }
pub struct Node {
    #[cfg_attr(rust_js, rust_js::flatten)]
    pub kind: Kind,
}
pub fn node() -> Node { Node { kind: Kind::Root } }
`,
    ),
  ).toContain("flattening `Kind`: a flattened field is a struct of named fields, a JS object, a `RefCell` of a `#[rust_js::tag]` enum, or an enum whose variants each have a field no other variant has, with at most one variant of no fields");
  // A variant whose every key another has.
  expect(
    refused(
      "flattened-keyed-shared",
      `pub enum Kind { Signal { value: u32 }, Computed { value: u32 } }
pub struct Node {
    #[cfg_attr(rust_js, rust_js::flatten)]
    pub kind: Kind,
}
pub fn node() -> Node { Node { kind: Kind::Signal { value: 1 } } }
`,
    ),
  ).toContain("flattening `Kind`: a flattened field is");
  // A key the parent has as its own field's.
  expect(
    refused(
      "flattened-keyed-shadowed",
      `${kind}pub struct Node {
    pub value: u32,
    #[cfg_attr(rust_js, rust_js::flatten)]
    pub kind: Kind,
}
pub fn node() -> Node { Node { value: 1, kind: Kind::Scope } }
`,
    ),
  ).toContain("flattening `Kind`, whose `value` is a key of its parent's own");
  // A type parameter given what no key tells, a number.
  expect(
    refused(
      "flattened-keyed-param-number",
      `pub struct Node<K> {
    pub flags: u32,
    #[cfg_attr(rust_js, rust_js::flatten)]
    pub kind: K,
}
pub fn node() -> u32 { let n = Node { flags: 1, kind: 2u32 }; n.flags }
`,
    ),
  ).toContain("flattening `u32` into `Node`: a flattened type parameter is given a struct of named fields, a JS object, or an enum whose variants each have a field no other variant has");
  // Given a keyed enum whose key the parent has.
  expect(
    refused(
      "flattened-keyed-param-shadowed",
      `${kind}pub struct Node<K> {
    pub current: u32,
    #[cfg_attr(rust_js, rust_js::flatten)]
    pub kind: K,
}
pub fn node() -> u32 { let n = Node { current: 1, kind: Kind::Scope }; n.current }
`,
    ),
  ).toContain("flattening `Kind` into `Node`: its `current` is a key of the parent's own");
});

// It's no object of its own: moved out or lent mutably whole, it would be
// the parent's object, with the parent's keys too.
test("a flattened keyed enum moved out or lent mutably whole is refused", () => {
  const node = `${kind}pub struct Node {
    pub flags: u32,
    #[cfg_attr(rust_js, rust_js::flatten)]
    pub kind: Kind,
}
`;
  expect(
    refused(
      "flattened-keyed-moved",
      `${node}pub fn take(node: Node) -> Kind { node.kind }
`,
    ),
  ).toContain("reading flattened props whole");
  expect(
    refused(
      "flattened-keyed-lent-mut",
      `${node}fn reset(kind: &mut Kind) { *kind = Kind::Scope; }
pub fn reset_node(node: &mut Node) { reset(&mut node.kind); }
`,
    ),
  ).toContain("reading flattened props whole");
  // Of a type parameter, whose keys generic code can't know to delete.
  expect(
    refused(
      "flattened-keyed-param-written",
      `pub struct Node<K> {
    pub flags: u32,
    #[cfg_attr(rust_js, rust_js::flatten)]
    pub kind: K,
}
pub fn replace<K>(node: &mut Node<K>, kind: K) { node.kind = kind; }
`,
    ),
  ).toContain("reading flattened props whole");
});

// Another crate reads which of a library's enums are keyed from its
// contract (library ABI 15): it makes, matches and flattens the library's
// as the library does, and gives its own enum to the library's flattened
// type parameter.
test("a library's keyed enum is keyed in its consumer, as its contract says", async () => {
  const dir = fixture("flattened-keyed-cross-crate");
  writeFileSync(
    join(dir, "dep.rs"),
    `pub struct Node<K> {
    pub flags: u32,
    #[cfg_attr(rust_js, rust_js::flatten)]
    pub kind: K,
}

${kind}
pub fn node<K>(flags: u32, kind: K) -> Node<K> {
    Node { flags, kind }
}

pub fn scope() -> Kind {
    Kind::Scope
}

pub fn signal(node: &Node<Kind>) -> Option<u32> {
    match &node.kind {
        Kind::Signal { current } => *current,
        _ => None,
    }
}
`,
  );
  const dep = runSync(
    [compiler, join(dir, "dep.rs"), "-o", join(dir, "dep", "lib.js"), "--library", "--manifest", join(dir, "dep", "lib.manifest.json"),
      "--", "--crate-name", "dep", `--emit=metadata=${join(dir, "libdep.rmeta")}`],
    root,
    600_000,
  );
  expect(dep.code, dep.stderr).toBe(0);
  expect(JSON.parse(readFileSync(join(dir, "dep", "lib.manifest.json"), "utf8")).library.keyed).toHaveLength(1);
  writeFileSync(
    join(dir, "app.rs"),
    `use dep::{Kind, Node};

pub enum Mode {
    Idle,
    Busy { since: u32 },
}

fn name(kind: &Kind) -> &'static str {
    match kind {
        Kind::Signal { .. } => "signal",
        Kind::Computed { .. } => "computed",
        Kind::Scope => "scope",
    }
}

fn three() -> u32 {
    3
}

pub fn run() -> String {
    let scope = dep::scope();
    let signal: Node<Kind> = dep::node(1, Kind::Signal { current: None });
    let computed = Node { flags: 2, kind: Kind::Computed { value: None, getter: three } };
    let busy: Node<Mode> = dep::node(4, Mode::Busy { since: 9 });
    let busy = match busy.kind {
        Mode::Idle => 0,
        Mode::Busy { since } => since,
    };
    format!("{} {} {} {:?} {busy}", name(&scope), name(&signal.kind), name(&computed.kind), dep::signal(&signal))
}
`,
  );
  const app = runSync(
    [compiler, join(dir, "app.rs"), "-o", join(dir, "app", "lib.js"), "--dependency", join(dir, "dep", "lib.manifest.json"),
      "--", "--crate-name", "app", "--extern", `dep=${join(dir, "libdep.rmeta")}`, "-L", dir],
    root,
    600_000,
  );
  expect(app.code, app.stderr).toBe(0);
  const js = readFileSync(join(dir, "app", "lib.js"), "utf8");
  expect(js).not.toContain("TAG");
  expect(js).toContain('"getter" in');
  expect((await import(join(dir, "app", "lib.js"))).run()).toBe("scope signal computed None 9");
});
