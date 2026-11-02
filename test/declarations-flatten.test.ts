// What a crate's declarations (ADR 0196) say of a flattened field's struct
// (ADR 0204): what the struct extends, where TypeScript can see it.

import { beforeAll, expect, test } from "bun:test";
import { readFileSync, writeFileSync } from "node:fs";
import { join } from "node:path";
import { buildReact, compiler, fixture, run, target } from "./support";

beforeAll(buildReact, 600_000);

// Another module's type of the crate's own, `pub(crate) struct Far`, which
// no module's declarations export: `any` to TypeScript, so the struct that
// flattens it extends nothing, never `extends any`, which TypeScript
// refuses, and has the rest's index signature (ADR 0204).
test("a flattened field of a type TypeScript can't see extends nothing", () => {
  const dir = fixture("declarations-flatten-unseen");
  writeFileSync(join(dir, "Cargo.toml"), '[package]\nname = "app"\nversion = "0.0.0"\nedition = "2024"\n');
  writeFileSync(
    join(dir, "lib.rs"),
    `mod far {
    #[derive(Default)]
    pub(crate) struct Far {
        pub id: Option<&'static str>,
    }
}

#[derive(Default)]
pub struct CardProps {
    pub title: Option<&'static str>,
    #[cfg_attr(rust_js, rust_js::flatten)]
    far: far::Far,
}

impl CardProps {
    pub fn id(&self) -> Option<&'static str> {
        self.far.id
    }
}

pub fn card(title: &'static str) -> CardProps {
    CardProps {
        title: Some(title),
        far: far::Far { id: Some("x") },
    }
}
`,
  );
  run([compiler, "--declarations", join(dir, "lib.rs"), "-o", join(dir, "lib.js")]);
  const declarations = readFileSync(join(dir, "lib.d.ts"), "utf8");
  expect(declarations).toContain("export interface CardProps {\n    title?: string | undefined;\n    [prop: string]: unknown;\n}");
  expect(declarations).not.toContain("extends");
});

// rd380's flattened state (ADR 1044) is its class's own keys: each a
// property of the class's declaration, the tag any variant's, and each
// other key what any variant holds there, `undefined` of one that holds
// nothing there, as the object has it.
test("a class's flattened state declares its keys", () => {
  const dir = fixture("declarations-flattened-state");
  writeFileSync(join(dir, "lib.rs"), `use std::cell::RefCell;

use js::Unknown;

#[cfg_attr(rust_js, rust_js::tag = "status")]
pub enum ChunkState {
    #[cfg_attr(rust_js, rust_js::name = "pending")]
    Pending {
        #[cfg_attr(rust_js, rust_js::nullable)]
        value: Option<Vec<&'static dyn Fn(Option<&'static Unknown>)>>,
        #[cfg_attr(rust_js, rust_js::nullable)]
        reason: Option<Vec<&'static dyn Fn(Option<&'static Unknown>)>>,
    },
    #[cfg_attr(rust_js, rust_js::name = "fulfilled")]
    Initialized {
        value: Option<&'static Unknown>,
        #[cfg_attr(rust_js, rust_js::nullable)]
        reason: Option<&'static Unknown>,
    },
    #[cfg_attr(rust_js, rust_js::name = "rejected")]
    Errored { reason: Option<&'static Unknown> },
}

#[cfg_attr(rust_js, rust_js::class)]
pub struct ReactPromise {
    #[cfg_attr(rust_js, rust_js::flatten)]
    pub state: RefCell<ChunkState>,
}

unsafe impl js::Defined for ReactPromise {}

#[cfg_attr(rust_js, rust_js::class)]
impl ReactPromise {
    fn new(state: ChunkState) -> Self {
        ReactPromise {
            state: RefCell::new(state),
        }
    }
}

pub fn create_pending_chunk() -> std::rc::Rc<ReactPromise> {
    std::rc::Rc::new(ReactPromise::new(ChunkState::Pending {
        value: None,
        reason: None,
    }))
}

pub fn status(chunk: &ReactPromise) -> bool {
    matches!(*chunk.state.borrow(), ChunkState::Pending { .. })
}

#[cfg_attr(rust_js, rust_js::tag = "kind")]
pub enum Step {
    #[cfg_attr(rust_js, rust_js::name = "counted")]
    Counted { n: u32 },
    #[cfg_attr(rust_js, rust_js::name = "named")]
    Named { name: String, n: u32 },
}

#[cfg_attr(rust_js, rust_js::class)]
pub struct Stepper {
    pub label: String,
    #[cfg_attr(rust_js, rust_js::flatten)]
    pub step: RefCell<Step>,
}

unsafe impl js::Defined for Stepper {}

#[cfg_attr(rust_js, rust_js::class)]
impl Stepper {
    fn new(label: String, step: Step) -> Self {
        Stepper {
            label,
            step: RefCell::new(step),
        }
    }
}

pub fn stepper(label: String) -> std::rc::Rc<Stepper> {
    std::rc::Rc::new(Stepper::new(label, Step::Counted { n: 1 }))
}

pub fn named(stepper: &Stepper) -> bool {
    matches!(*stepper.step.borrow(), Step::Named { .. })
}
`);
  run([compiler, "--declarations", join(dir, "lib.rs"), "-o", join(dir, "lib.js"), "--", "--extern", `js=${join(target, "libjs.rmeta")}`, "-L", target]);
  const declarations = readFileSync(join(dir, "lib.d.ts"), "utf8");
  expect(declarations).toContain(
    'declare class ReactPromise {\n    status: "pending" | "fulfilled" | "rejected";\n    value: unknown;\n    reason: unknown;\n}',
  );
  expect(declarations).toContain(
    'declare class Stepper {\n    kind: "counted" | "named";\n    n: number;\n    name: undefined | string;\n    label: string;\n}',
  );
});
