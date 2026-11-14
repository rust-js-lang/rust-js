// A component whose props are another module's or crate's, imported,
// renamed, re-exported or aliased, as an ordinary wrapper of a library's
// component takes them (#1794): its props are built by their struct's own
// companion (ADR 0213), as the library's component's are, so a flattened
// field's attributes are given flat and an `Option` left out, not a struct
// literal that lacks them. Each test is a program of its own, compiled
// and run.

import { beforeAll, expect, test } from "bun:test";
import { mkdirSync, readFileSync, writeFileSync } from "node:fs";
import { join } from "node:path";
import { renderToStaticMarkup } from "react-dom/server";
import { buildReact, compiler, fixture, run, target } from "./support";

beforeAll(buildReact, 600_000);

const react = ["--extern", `react=${join(target, "libreact.rmeta")}`, "-L", target];

function written(dir: string, files: Record<string, string>) {
  for (const [name, body] of Object.entries(files)) {
    mkdirSync(join(dir, name, ".."), { recursive: true });
    writeFileSync(join(dir, name), body);
  }
}

// The issue's program, of a library of its own as Next's `ImageProps` is:
// a `Default` struct of a type parameter, its `<img>` attributes flattened,
// and its component; a wrapper in another crate takes it, imported, and
// spreads it on the library's component.
const picture = `#![allow(non_snake_case)]
use react::attributes::ImgHTMLAttributes;
use react::{JSX, jsx};

#[derive(Default)]
pub struct PictureProps<'a, S = &'a str> {
    pub src: S,
    pub alt: &'a str,
    pub fill: Option<bool>,
    #[rust_js::flatten]
    pub img: ImgHTMLAttributes<'a>,
}

pub trait IntoPictureSrc {}
impl IntoPictureSrc for &str {}

pub fn Picture<S: IntoPictureSrc>(props: PictureProps<'_, S>) -> JSX::Element {
    let PictureProps { alt, fill, img, .. } = props;
    jsx! { <img {...img} alt={Some(alt)} data-fill={fill.unwrap_or_default()} /> }
}
`;

test("an ordinary component takes another crate's props, imported, and is given their flattened attributes flat", async () => {
  const dir = fixture("jsx-imported-props");
  written(dir, {
    "ui/lib.rs": picture,
    "app/lib.rs": `#![allow(non_snake_case)]
use react::{JSX, jsx};
use ui::{Picture, PictureProps};

pub fn DeferredPicture<S: ui::IntoPictureSrc + Copy>(props: PictureProps<'_, S>) -> JSX::Element {
    jsx! { <Picture {...props} /> }
}

pub fn Example() -> JSX::Element {
    jsx! {
        <DeferredPicture
            src="/example.png"
            alt="Example"
            fill={Some(true)}
            fetchPriority={Some("low")}
        />
    }
}
`,
  });
  run([compiler, join(dir, "ui/lib.rs"), "-o", join(dir, "ui/js/lib.jsx"), "--library", "--manifest", join(dir, "ui/js/lib.manifest.json"),
    "--", "--crate-name", "ui", `--emit=metadata=${join(dir, "ui/js/libui.rmeta")}`, ...react]);
  run([compiler, join(dir, "app/lib.rs"), "-o", join(dir, "app/js/lib.jsx"), "--dependency", join(dir, "ui/js/lib.manifest.json"),
    "--", "--crate-name", "app", "--extern", `ui=${join(dir, "ui/js/libui.rmeta")}`, ...react]);
  const jsx = readFileSync(join(dir, "app/js/lib.jsx"), "utf8");
  expect(jsx).toContain('<DeferredPicture src="/example.png" alt="Example" fill fetchPriority="low" />');
  expect(jsx).toContain("<Picture {...props} />");
});

// One crate's modules: a props struct of another module, by its path, a
// `use`, a `use .. as`, a `pub use` and a type alias of it, its `Option`
// left out and its required field said missing.
const cards = `#![allow(non_snake_case)]
use react::{JSX, jsx};

pub mod types {
    pub struct CardProps<'a> {
        pub title: &'a str,
        pub subtitle: Option<&'a str>,
    }

    pub type Titled<'a> = CardProps<'a>;
}

pub mod shown {
    pub use super::types::CardProps;
}

pub mod cards {
    use super::shown;
    use super::types::{self, CardProps, Titled};
    use super::types::CardProps as Props;
    use react::{JSX, jsx};

    pub fn Card(props: CardProps<'_>) -> JSX::Element {
        jsx! { <h2 title={props.subtitle}>{props.title}</h2> }
    }

    pub fn Renamed(props: Props<'_>) -> JSX::Element {
        jsx! { <h3>{props.title}</h3> }
    }

    pub fn Pathed(props: types::CardProps<'_>) -> JSX::Element {
        jsx! { <h4>{props.title}</h4> }
    }

    pub fn Reexported(props: shown::CardProps<'_>) -> JSX::Element {
        jsx! { <h5>{props.title}</h5> }
    }

    pub fn Aliased(props: Titled<'_>) -> JSX::Element {
        jsx! { <h6>{props.title}</h6> }
    }
}

use cards::{Aliased, Card, Pathed, Reexported, Renamed};
`;

test("a component takes its crate's props of another module, used, renamed, pathed, re-exported or aliased", async () => {
  const dir = fixture("jsx-module-props");
  written(dir, {
    "lib.rs": `${cards}
pub fn All() -> JSX::Element {
    jsx! {
        <>
            <Card title="a" />
            <Card title="b" subtitle={Some("sub")} />
            <Renamed title="c" />
            <Pathed title="d" />
            <Reexported title="e" />
            <Aliased title="f" />
        </>
    }
}
`,
  });
  run([compiler, join(dir, "lib.rs"), "-o", join(dir, "lib.js"), "--", ...react]);
  const lib = await import(join(dir, "lib.jsx"));
  expect(renderToStaticMarkup(lib.All())).toBe('<h2>a</h2><h2 title="sub">b</h2><h3>c</h3><h4>d</h4><h5>e</h5><h6>f</h6>');
});

test("an imported props struct's required field left out is said missing, of the struct", () => {
  const dir = fixture("jsx-module-props-missing");
  written(dir, { "lib.rs": `${cards}\npub fn Missing() -> JSX::Element {\n    jsx! { <Card subtitle={Some("x")} /> }\n}\n` });
  const failed = Bun.spawnSync([compiler, join(dir, "lib.rs"), "-o", join(dir, "lib.js"), "--", ...react]);
  expect([failed.exitCode === 0, failed.stderr.toString()]).toEqual([false, expect.stringContaining("missing prop `title` of `CardProps`")]);
});

// A struct a field of which is less visible than it: no other module can
// build it, and JSX doesn't either, past rustc's privacy.
test("props of another module with a private field are refused, saying to make the fields as visible", () => {
  const dir = fixture("jsx-module-props-private");
  written(dir, {
    "lib.rs": `#![allow(non_snake_case)]
use react::{JSX, jsx};

pub mod types {
    pub struct Secret {
        pub shown: u32,
        hidden: u32,
    }

    impl Secret {
        pub fn hidden(&self) -> u32 {
            self.hidden
        }
    }
}

pub fn Show(props: types::Secret) -> JSX::Element {
    jsx! { <p>{props.shown + props.hidden()}</p> }
}

pub fn Shown() -> JSX::Element {
    jsx! { <Show shown={1} /> }
}
`,
  });
  const failed = Bun.spawnSync([compiler, join(dir, "lib.rs"), "-o", join(dir, "lib.js"), "--", ...react]);
  expect([failed.exitCode === 0, failed.stderr.toString()]).toEqual([
    false,
    expect.stringContaining("`jsx!` builds `Secret` as a component's props only where each of its fields is as visible as it"),
  ]);
});

// A module's own macro of a struct's name is the module's, an exported
// one at the crate's root, where a companion of the name would be defined
// twice: the struct has none, and its own components build it as a
// literal, as before.
test("a struct of a macro's name keeps the macro, its props built as a literal", async () => {
  const dir = fixture("jsx-props-macro-name");
  written(dir, {
    "lib.rs": `#![allow(non_snake_case)]
use react::{JSX, jsx};

pub struct Badge {
    pub label: &'static str,
}

#[macro_export]
macro_rules! Badge {
    ($label:expr) => {
        Badge { label: $label }
    };
}

pub fn Shown(props: Badge) -> JSX::Element {
    jsx! { <b>{props.label}</b> }
}

pub fn All() -> JSX::Element {
    let made = Badge!("m");
    jsx! { <><Shown label="a" /><Shown label={made.label} /></> }
}
`,
  });
  run([compiler, join(dir, "lib.rs"), "-o", join(dir, "lib.js"), "--", ...react]);
  const lib = await import(join(dir, "lib.jsx"));
  expect(renderToStaticMarkup(lib.All())).toBe("<b>a</b><b>m</b>");
});

// A struct a `Default` is written for is built by its companion in its own
// module, where a private field of it is seen, as React's attributes are,
// flattened (ADR 0213): the refusal is of another module's.
test("a struct of a private field with a Default written for it is flattened in its own module", async () => {
  const dir = fixture("jsx-props-written-default");
  written(dir, {
    "lib.rs": `#![allow(non_snake_case)]
use react::{JSX, jsx};

pub struct Look {
    pub color: Option<&'static str>,
    weight: Option<u32>,
}

impl Default for Look {
    fn default() -> Self {
        Look { color: None, weight: Some(700) }
    }
}

impl Look {
    pub fn weight(&self) -> u32 {
        self.weight.unwrap_or(400)
    }
}

pub struct BadgeProps {
    pub label: &'static str,
    #[rust_js::flatten]
    pub look: Look,
}

pub fn Badge(props: BadgeProps) -> JSX::Element {
    jsx! { <b data-color={props.look.color} data-weight={props.look.weight()}>{props.label}</b> }
}

pub fn All() -> JSX::Element {
    jsx! { <Badge label="x" color={Some("red")} /> }
}
`,
  });
  run([compiler, join(dir, "lib.rs"), "-o", join(dir, "lib.js"), "--", ...react]);
  const lib = await import(join(dir, "lib.jsx"));
  expect(renderToStaticMarkup(lib.All())).toBe('<b data-color="red" data-weight="700">x</b>');
});
