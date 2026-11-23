// A doc comment on a `pub use` of types alone (#1869): it documents the
// re-export, which only the declarations have, `export type { Size as
// Measure };`, so it's that statement's JSDoc there, and in the JS, where
// the re-export is nothing, it's nowhere: not the next function's, as the
// deep-chain harness's kit gave `cn` its `DropdownProps`' doc.

import { expect, test } from "bun:test";
import { readFileSync, writeFileSync } from "node:fs";
import { join } from "node:path";
import { compiler, fixture, run } from "./support";

const program = `/// How big a thing is.
pub struct Size {
    pub width: u32,
}

// As a kit names it.
/// A measure is a size, by the name a kit gives it.
pub use Size as Measure;

/// The classes given, joined.
pub fn cn(parts: &[&str]) -> String {
    parts.join(" ")
}

pub fn width(size: &Measure) -> u32 {
    size.width
}
`;

test("a doc comment on a pub use of types is its re-export's, nowhere in the JS", () => {
  const dir = fixture("reexport-docs");
  writeFileSync(join(dir, "lib.rs"), program);
  run([compiler, "--declarations", join(dir, "lib.rs"), "-o", join(dir, "lib.js")]);
  const js = readFileSync(join(dir, "lib.js"), "utf8");
  expect(js).toContain("/** The classes given, joined. */\nexport function cn(parts) {");
  expect(js).not.toContain("measure is a size");
  const declarations = readFileSync(join(dir, "lib.d.ts"), "utf8");
  expect(declarations).toContain("// As a kit names it.\n/** A measure is a size, by the name a kit gives it. */\nexport type { Size as Measure };");
  expect(declarations).toContain("/** The classes given, joined. */\nexport function cn(parts: readonly string[]): string;");
});

// A `pub use` of a type and a value: its doc comment is its value's
// statement's, the one the JS has, `export { width } from`, and the type's
// `export type` has none of it.
test("a doc comment on a pub use of a type and a value is its value's", () => {
  const dir = fixture("reexport-docs-mixed");
  writeFileSync(join(dir, "lib.rs"), `pub mod parts {
    pub struct Size {
        pub width: u32,
    }

    pub fn width(size: &Size) -> u32 {
        size.width
    }
}

/// What a kit gives of its parts: a size, and its width.
pub use parts::{Size as Measure, width};

/// The classes given, joined.
pub fn cn(parts: &[&str]) -> String {
    parts.join(" ")
}
`);
  run([compiler, "--declarations", join(dir, "lib.rs"), "-o", join(dir, "lib.js")]);
  const js = readFileSync(join(dir, "lib.js"), "utf8");
  expect(js).toContain('/** What a kit gives of its parts: a size, and its width. */\nexport { width } from "./parts.js";');
  expect(js).toContain("/** The classes given, joined. */\nexport function cn(parts) {");
  const declarations = readFileSync(join(dir, "lib.d.ts"), "utf8");
  expect(declarations).toContain('\nexport type { Size as Measure } from "./parts.js";');
  expect(declarations).not.toContain("*/\nexport type { Size as Measure }");
  expect(declarations).toContain('/** What a kit gives of its parts: a size, and its width. */\nexport { width } from "./parts.js";');
});

// A `pub use` of a value alone: its comments, `//` and its doc comment,
// are its declaration's in the .d.ts as its statement's in the JS (#1880).
test("a doc comment on a pub use of a value is its declaration's too", () => {
  const dir = fixture("reexport-docs-value");
  writeFileSync(join(dir, "lib.rs"), `pub mod parts {
    pub fn width(x: u32) -> u32 {
        x
    }

    pub fn height(x: u32) -> u32 {
        x
    }
}

// Kept as a kit names it.
/// The width of a part.
pub use parts::width;

pub use parts::height;
`);
  run([compiler, "--declarations", join(dir, "lib.rs"), "-o", join(dir, "lib.js")]);
  const documented = '// Kept as a kit names it.\n/** The width of a part. */\nexport { width } from "./parts.js";\n';
  expect(readFileSync(join(dir, "lib.js"), "utf8")).toContain(documented);
  const declarations = readFileSync(join(dir, "lib.d.ts"), "utf8");
  expect(declarations).toContain(documented);
  expect(declarations).toContain('*/\nexport { width } from "./parts.js";\n\nexport { height } from "./parts.js";');
});

// A tuple struct and a unit struct are types alone too, their constructors
// no JS a `use` re-exports: its doc comment is nowhere in the JS.
test("a doc comment on a pub use of structs with constructors is nowhere in the JS", () => {
  const dir = fixture("reexport-docs-constructors");
  writeFileSync(join(dir, "lib.rs"), `pub mod parts {
    pub struct Px(pub u32);

    pub struct Unit;

    pub fn px(value: u32) -> Px {
        Px(value)
    }
}

/// Pixels and a unit.
pub use parts::{Px, Unit};

/// The classes given, joined.
pub fn cn(parts: &[&str]) -> String {
    parts.join(" ")
}
`);
  run([compiler, "--declarations", join(dir, "lib.rs"), "-o", join(dir, "lib.js")]);
  const js = readFileSync(join(dir, "lib.js"), "utf8");
  expect(js).toContain("/** The classes given, joined. */\nexport function cn(parts) {");
  expect(js).not.toContain("Pixels and a unit");
});

// A `pub use` of a trait, which has no JS, re-exports the type of its
// dictionaries, `interface Shape<Self>`, by the name the `use` gives it
// (#2095), and its doc comment is that `export type`'s, nowhere in the JS,
// as a type's is (#1869).
test("a pub use of a trait re-exports its interface, with its doc comment", () => {
  const dir = fixture("reexport-docs-trait");
  writeFileSync(join(dir, "shapes.rs"), `/// A shape.
pub trait Shape {
    fn area(&self) -> f64;
}

pub struct Square {
    pub side: f64,
}

impl Shape for Square {
    fn area(&self) -> f64 {
        self.side * self.side
    }
}

pub fn twice<S: Shape>(s: &S) -> f64 {
    s.area() * 2.0
}
`);
  writeFileSync(join(dir, "lib.rs"), `mod shapes;

/// Shapes, by a kit's name.
pub use shapes::Shape as Form;

/// Twice a shape's area.
pub use shapes::{Square, twice};
`);
  run([compiler, "--declarations", join(dir, "lib.rs"), "-o", join(dir, "lib.js")]);
  const js = readFileSync(join(dir, "lib.js"), "utf8");
  expect(js).not.toContain("by a kit's name");
  expect(js).toContain('/** Twice a shape\'s area. */\nexport { twice } from "./shapes.js";');
  const declarations = readFileSync(join(dir, "lib.d.ts"), "utf8");
  expect(declarations).toContain('/** Shapes, by a kit\'s name. */\nexport type { Shape as Form } from "./shapes.js";');
  expect(declarations).toContain('export type { Square } from "./shapes.js";');
});
