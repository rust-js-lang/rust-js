// What a crate's declarations (ADR 0196) say of a flattened field's struct
// (ADR 0204): what the struct extends, where TypeScript can see it.

import { expect, test } from "bun:test";
import { readFileSync, writeFileSync } from "node:fs";
import { join } from "node:path";
import { compiler, fixture, run } from "./support";

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
