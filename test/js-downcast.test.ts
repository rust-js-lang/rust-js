// A checked downcast (#1866's follow-up, ADR 1159): `js::downcast::<T>(value)`
// is the value where it's a `T`, a class JS can test it by, `value
// instanceof HTMLButtonElement ? value : undefined`, else `None`, which
// TypeScript's `closest<E extends Element>(..): E | null` would cast
// unchecked. It adds the instanceof upstream doesn't write, false of
// another realm's object; of a type parameter, whose class JS doesn't
// know, it's refused.

import { beforeAll, expect, test } from "bun:test";
import { readFileSync, writeFileSync } from "node:fs";
import { join } from "node:path";
import { buildReact, compiler, fixture, root, run, target } from "./support";

beforeAll(buildReact, 600_000);

const withWeb = ["--", "--extern", `webapi=${join(target, "libwebapi.rmeta")}`, "--extern", `js=${join(target, "libjs.rmeta")}`, "-L", target];

const program = `use webapi::{Element, HTMLButtonElement};

/// The button an element is, or none.
pub fn button_of(element: &'static Element) -> Option<&'static HTMLButtonElement> {
    js::downcast::<HTMLButtonElement>(element)
}

/// Whether the first is a disabled button, read once.
pub fn first_disabled(elements: &[&'static Element]) -> bool {
    js::downcast::<HTMLButtonElement>(pick(elements)).is_some_and(|button| button.disabled())
}

fn pick(elements: &[&'static Element]) -> &'static Element {
    elements[0]
}
`;

test("js::downcast is the value where it's the class's instance, else None", async () => {
  const dir = fixture("js-downcast");
  writeFileSync(join(dir, "lib.rs"), program);
  run([compiler, join(dir, "lib.rs"), "-o", join(dir, "lib.js"), ...withWeb]);
  const js = readFileSync(join(dir, "lib.js"), "utf8");
  expect(js).toContain("return element instanceof HTMLButtonElement ? element : undefined;");
  // What it tests is read once, held where it runs code.
  expect(js).toContain("  const value = pick(elements);\n  const button = value instanceof HTMLButtonElement ? value : undefined;\n");
  // Rust's answers, of classes as a page has them.
  const host = globalThis as Record<string, unknown>;
  class Element {}
  class HTMLButtonElement extends Element {
    constructor(public disabled: boolean) {
      super();
    }
  }
  host.HTMLButtonElement = HTMLButtonElement;
  try {
    const { buttonOf, firstDisabled } = await import(join(dir, "lib.js"));
    const button = new HTMLButtonElement(true);
    const div = new Element();
    expect([buttonOf(button), buttonOf(div)]).toEqual([button, undefined]);
    expect([firstDisabled([button]), firstDisabled([div]), firstDisabled([new HTMLButtonElement(false)])]).toEqual([true, false, false]);
  } finally {
    delete host.HTMLButtonElement;
  }
});

test("js::downcast to a type parameter is refused, its class unknown to JS", () => {
  const dir = fixture("js-downcast-refused");
  writeFileSync(join(dir, "lib.rs"), `use webapi::Element;

pub fn of<T: js::Classed>(element: &'static Element) -> Option<&'static T> {
    js::downcast::<T>(element)
}
`);
  const refused = Bun.spawnSync([compiler, join(dir, "lib.rs"), "-o", join(dir, "lib.js"), ...withWeb], { cwd: root, stderr: "pipe", timeout: 60_000 });
  expect(refused.exitCode).not.toBe(0);
  expect(refused.stderr.toString()).toContain("`js::downcast::<T>` of a type parameter");
});
