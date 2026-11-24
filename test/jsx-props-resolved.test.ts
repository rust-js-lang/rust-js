// A component's props of a library's type a glob or a prelude imports
// (ADR 0213, amended; Codex's prototype, rust-js-lang/vgpu-in-rust@143d724):
// before rustc resolves a name, the syntax pass predicts whether the props
// type has a companion, of the module's `Scope`; after expansion, rustc's
// names say whether it has, a macro of the struct's name beside it that
// rust-js made, `#[rust_js::jsx_props]`; where the prediction was wrong,
// the crate is compiled once more with that component's answer, and only
// then. Diagnostics of a compile that's replayed are dropped, so each is
// said once.

import { beforeAll, expect, test } from "bun:test";
import { existsSync, mkdirSync, readFileSync, writeFileSync } from "node:fs";
import { join } from "node:path";
import { runSync } from "./child";
import { buildReact, compiler, fixture, root, run, target } from "./support";

beforeAll(buildReact, 600_000);

const react = ["--extern", `react=${join(target, "libreact.rmeta")}`, "-L", target];

function written(dir: string, files: Record<string, string>) {
  for (const [name, body] of Object.entries(files)) {
    mkdirSync(join(dir, name, ".."), { recursive: true });
    writeFileSync(join(dir, name), body);
  }
}

// A library of a component whose props flatten an `<img>`'s attributes, and
// a prelude that re-exports both, as UI kits give theirs.
const ui = `#![allow(non_snake_case)]
use react::attributes::ImgHTMLAttributes;
use react::{JSX, jsx};

#[derive(Default)]
pub struct PictureProps<'a> {
    pub alt: &'a str,
    #[rust_js::flatten]
    pub img: ImgHTMLAttributes<'a>,
}

pub fn Picture(props: PictureProps<'_>) -> JSX::Element {
    let PictureProps { alt, img } = props;
    jsx! { <img {...img} alt={Some(alt)} /> }
}

pub mod prelude {
    pub use super::{Picture, PictureProps};
}
`;

/** `app`, compiled against `ui`, a library: its JSX, its stderr, and the
 * components whose props a replay built by their companion. */
function compiled(name: string, app: string): { jsx: string; stderr: string; replayed: string[] } {
  const dir = fixture(name);
  written(dir, { "ui/lib.rs": ui, "app/lib.rs": app });
  run([compiler, join(dir, "ui/lib.rs"), "-o", join(dir, "ui/js/lib.jsx"), "--library", "--manifest", join(dir, "ui/js/lib.manifest.json"),
    "--", "--crate-name", "ui", `--emit=metadata=${join(dir, "ui/js/libui.rmeta")}`, ...react]);
  const replays = join(dir, "replayed.txt");
  const built = runSync([compiler, "--props-replayed", replays, join(dir, "app/lib.rs"), "-o", join(dir, "app/js/lib.jsx"), "--dependency", join(dir, "ui/js/lib.manifest.json"),
    "--", "--crate-name", "app", "--extern", `ui=${join(dir, "ui/js/libui.rmeta")}`, ...react], root, 120_000);
  expect([built.code, built.stderr.includes("error")]).toEqual([0, false]);
  return {
    jsx: readFileSync(join(dir, "app/js/lib.jsx"), "utf8"),
    stderr: built.stderr,
    replayed: existsSync(replays) ? readFileSync(replays, "utf8").split("\n").filter(Boolean) : [],
  };
}

const wrapper = (imports: string) => `#![allow(non_snake_case)]
use react::{JSX, jsx};
${imports}

pub fn Framed(props: PictureProps<'_>) -> JSX::Element {
    jsx! { <figure><Picture {...props} /></figure> }
}

pub fn Example() -> JSX::Element {
    jsx! { <Framed alt="Example" fetchPriority={Some("low")} /> }
}
`;

test("a library's props a glob imports are built by their companion, by one replay", () => {
  const { jsx, replayed } = compiled("jsx-props-glob", wrapper("use ui::*;"));
  expect(jsx).toContain('<Framed alt="Example" fetchPriority="low" />');
  expect(replayed).toEqual(["Framed"]);
});

test("a prelude's, the same", () => {
  const { jsx, replayed } = compiled("jsx-props-prelude", wrapper("use ui::prelude::*;"));
  expect(jsx).toContain('<Framed alt="Example" fetchPriority="low" />');
  expect(replayed).toEqual(["Framed"]);
});

test("a use that names them is predicted, and compiled once", () => {
  const { jsx, replayed } = compiled("jsx-props-named", wrapper("use ui::{Picture, PictureProps};"));
  expect(jsx).toContain('<Framed alt="Example" fetchPriority="low" />');
  expect(replayed).toEqual([]);
});

// Named by their crate's path, no `use` of them: predicted by the crate
// the build is given, so compiled once (#1979).
test("a path of their crate that names them is predicted, and compiled once", () => {
  const { jsx, replayed } = compiled("jsx-props-pathed", wrapper("use ui::Picture;").replace("props: PictureProps<'_>", "props: ui::PictureProps<'_>"));
  expect(jsx).toContain('<Framed alt="Example" fetchPriority="low" />');
  expect(replayed).toEqual([]);
});

test("a replayed compile's warnings are said once", () => {
  const { stderr, replayed } = compiled("jsx-props-warned", `${wrapper("use ui::*;")}
pub fn spare() -> u32 {
    (2)
}
`);
  expect(replayed).toEqual(["Framed"]);
  // An early lint's, which the first compile said before it was replayed.
  expect(stderr.split("unnecessary parentheses").length - 1).toBe(1);
});

// The module's own props are predicted by their companion beside them, so
// a crate of them compiles once, its prediction right.
test("the module's own props are predicted, and compiled once", () => {
  const { jsx, replayed } = compiled("jsx-props-own", `#![allow(non_snake_case)]
use react::attributes::ImgHTMLAttributes;
use react::{JSX, jsx};

#[derive(Default)]
pub struct FrameProps<'a> {
    pub alt: &'a str,
    #[rust_js::flatten]
    pub img: ImgHTMLAttributes<'a>,
}

pub fn Frame(props: FrameProps<'_>) -> JSX::Element {
    let FrameProps { alt, img } = props;
    jsx! { <img {...img} alt={Some(alt)} /> }
}

pub fn Example() -> JSX::Element {
    jsx! { <Frame alt="Example" fetchPriority={Some("low")} /> }
}
`);
  expect(jsx).toContain('<Frame alt="Example" fetchPriority="low" />');
  expect(replayed).toEqual([]);
});
