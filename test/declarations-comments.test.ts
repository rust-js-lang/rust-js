// What a crate's declarations (ADR 0196) carry of the comments written
// before an item: its `//` comments, as written, past its attributes, and
// its doc comment as JSDoc after them (ADR 0444).

import { expect, test } from "bun:test";
import { readFileSync, writeFileSync } from "node:fs";
import { join } from "node:path";
import { compiler, fixture, run } from "./support";

// React's ReactFiberLane and ReactFiberSuspenseComponent, ported: upstream's
// comments are on its Flow types, so the port's types carry them in `//`,
// a type's, a field's past its `#[cfg_attr(..)]`, and one a blank line is
// written after, `// TODO: Ideally these types would be opaque ..` before
// `pub type Lanes`. Not the file's license, which its header is.
test("declarations carry the comments written before an item", () => {
  const dir = fixture("declarations-comments");
  writeFileSync(
    join(dir, "lib.rs"),
    `// Copyright (c) Meta Platforms, Inc. and affiliates.

use std::cell::Cell;

// TODO: Ideally these types would be opaque but that doesn't work well with
// our reconciler fork infra, since these leak into non-reconciler packages.

pub type Lanes = i32;

// A null SuspenseState represents an unsuspended normal Suspense boundary.
pub struct SuspenseState {
    // If this boundary is still dehydrated, we store the SuspenseInstance
    // here to indicate that it is dehydrated (flag).
    #[cfg_attr(rust_js, rust_js::nullable)]
    pub dehydrated: Option<u32>,
    pub retry_lane: Cell<Lanes>, // Not a comment before an item.
    // A comment before its doc comment.
    /// Whether it's documented.
    pub documented: bool,
}

pub fn lanes(state: &SuspenseState) -> Lanes {
    state.retry_lane.get()
}
`,
  );
  run([compiler, "--declarations", join(dir, "lib.rs"), "-o", join(dir, "lib.js")]);
  const declarations = readFileSync(join(dir, "lib.d.ts"), "utf8");
  expect(declarations).toContain(
    "// TODO: Ideally these types would be opaque but that doesn't work well with\n// our reconciler fork infra, since these leak into non-reconciler packages.\n\nexport type Lanes = number;\n",
  );
  expect(declarations).toContain(
    "// A null SuspenseState represents an unsuspended normal Suspense boundary.\nexport interface SuspenseState {\n    // If this boundary is still dehydrated, we store the SuspenseInstance\n    // here to indicate that it is dehydrated (flag).\n    dehydrated?: number | null | undefined;\n    retryLane: Lanes;\n    // A comment before its doc comment.\n    /** Whether it's documented. */\n    documented: boolean;\n}",
  );
  expect(declarations).toContain("\nexport function lanes(state: SuspenseState): Lanes;");
  expect(declarations.match(/Copyright/g)).toHaveLength(1);
  // A file whose first item is right after its license: the header's.
  writeFileSync(join(dir, "lib.rs"), "// Copyright (c) Meta Platforms, Inc. and affiliates.\n\npub type Lane = i32;\n\npub fn one() -> Lane {\n    1\n}\n");
  run([compiler, "--declarations", join(dir, "lib.rs"), "-o", join(dir, "lib.js")]);
  const first = readFileSync(join(dir, "lib.d.ts"), "utf8");
  expect(first.match(/Copyright/g)).toHaveLength(1);
  expect(first).toContain("affiliates.\n\nexport type Lane = number;");
});

// React port gap rd67: a block doc comment, `/** .. */`, as upstream's
// JSDoc, is the JSDoc of its lines, as rustdoc reads them: not each line's
// star starred again, `* * A block ..`, nor its first and last empty lines
// kept.
test("a block doc comment is its lines' JSDoc", () => {
  const dir = fixture("declarations-block-docs");
  writeFileSync(
    join(dir, "lib.rs"),
    `/**
 * A block doc comment, as upstream's JSDoc,
 * across lines.
 */
pub fn documented() {}

/// A line doc,
///
/// across lines.
pub fn lined() {}
`,
  );
  run([compiler, "--declarations", join(dir, "lib.rs"), "-o", join(dir, "lib.js")]);
  const declarations = readFileSync(join(dir, "lib.d.ts"), "utf8");
  expect(declarations).toContain("/**\n * A block doc comment, as upstream's JSDoc,\n * across lines.\n */\nexport function documented(): void;\n");
  expect(declarations).toContain("/**\n * A line doc,\n *\n * across lines.\n */\nexport function lined(): void;\n");
});
