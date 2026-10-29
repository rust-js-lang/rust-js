// What a crate's declarations (ADR 0196) say of a struct whose impl binds
// methods of the JS object it is, `#[rust_js::link_name = "isPersistent"]
// pub fn is_persistent(&self) -> bool`: each is its interface's method, as
// its fields are its properties, so a TypeScript caller may call what a
// Rust one does.

import { beforeAll, expect, test } from "bun:test";
import { readFileSync, writeFileSync } from "node:fs";
import { join } from "node:path";
import { buildReact, compiler, fixture, run, target } from "./support";

beforeAll(buildReact, 600_000);

// React's SyntheticEvent, ported (rd144): its prototype's functions bound
// as methods of the event type, which upstream's BaseSyntheticEvent
// declares, `isPersistent: () => boolean`.
test("a struct's bound methods are its interface's", () => {
  const dir = fixture("declarations-bound-methods");
  writeFileSync(
    join(dir, "lib.rs"),
    `use std::cell::Cell;

pub struct SyntheticEvent {
    pub default_prevented: Cell<bool>,
    pub is_default_prevented: Cell<fn() -> bool>,
}

unsafe impl js::Defined for SyntheticEvent {}

impl SyntheticEvent {
    /// The prototype's \`preventDefault\`.
    #[cfg_attr(rust_js, rust_js::link_name = "preventDefault")]
    pub fn prevent_default(&self) {
        unreachable!()
    }

    /// The prototype's \`isPersistent\`.
    #[cfg_attr(rust_js, rust_js::link_name = "isPersistent")]
    pub fn is_persistent(&self) -> bool {
        unreachable!()
    }

    #[cfg_attr(rust_js, rust_js::link_name = "hidden")]
    fn hidden(&self) -> bool {
        unreachable!()
    }

    pub fn is_settled(&self) -> bool {
        self.default_prevented.get() || self.is_persistent()
    }
}

pub fn submit(event: &'static SyntheticEvent) -> bool {
    event.prevent_default();
    event.is_persistent() && (event.is_default_prevented.get())() && !event.hidden()
}
`,
  );
  run([compiler, "--declarations", join(dir, "lib.rs"), "-o", join(dir, "lib.js"), "--", "--extern", `js=${join(target, "libjs.rmeta")}`, "-L", target]);
  const declarations = readFileSync(join(dir, "lib.d.ts"), "utf8");
  expect(declarations).toContain(
    "export interface SyntheticEvent {\n    defaultPrevented: boolean;\n    isDefaultPrevented: () => boolean;\n    /** The prototype's `preventDefault`. */\n    preventDefault(): void;\n    /** The prototype's `isPersistent`. */\n    isPersistent(): boolean;\n}",
  );
});
