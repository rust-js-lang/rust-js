// What React keeps and gives a function again: a component's props, which
// it renders again with, twice in <StrictMode>; a reducer's action, an
// initializer's argument, which it calls again with (#1723, ADR 0020,
// amended). Rust's function owns what it's given by value, and its next
// call gets its own; so what it changes in place of one, or of a part it
// moved out, is copied first, and each renders Rust's answer under
// StrictMode, in real React.
import { beforeAll, expect, test } from "bun:test";
import { readFileSync, writeFileSync } from "node:fs";
import { join } from "node:path";
import { runSync } from "./child";
import { buildReact, compiler, fixture, root, run, target, bunTest } from "./support";

beforeAll(buildReact, 600_000);

const flags = (dir: string) => [join(dir, "lib.rs"), "-o", join(dir, "lib.js"), "--", "--extern", `react=${join(target, "libreact.rmeta")}`, "-L", target];

const program = `#![allow(non_snake_case)]
use react::{ForwardRefExoticComponent, ForwardedRef, JSX, forward_ref, jsx, use_reducer, use_reducer_with, use_state};

pub struct Point {
    pub x: f64,
}

pub struct DotProps {
    pub at: Point,
}

/// Its props, a part moved out and changed.
pub fn Moved(props: DotProps) -> JSX::Element {
    let mut at = props.at;
    at.x += 1.0;
    jsx! { <i>{at.x}</i> }
}

/// Its props, changed in place themselves.
pub fn Changed(mut props: DotProps) -> JSX::Element {
    props.at.x += 1.0;
    jsx! { <i>{props.at.x}</i> }
}

/// Its props taken apart where they're given, a part changed.
pub fn Apart(DotProps { mut at }: DotProps) -> JSX::Element {
    at.x += 1.0;
    jsx! { <i>{at.x}</i> }
}

/// Its props, given on unchanged: no copy.
pub fn Passed(props: DotProps) -> JSX::Element {
    jsx! { <Moved at={props.at} /> }
}

pub struct ListProps {
    pub items: Vec<Point>,
}

/// A \`Vec\` of its props, its items changed.
pub fn Shifted(props: ListProps) -> JSX::Element {
    let total: f64 = props
        .items
        .into_iter()
        .map(|mut point| {
            point.x += 1.0;
            point.x
        })
        .sum();
    jsx! { <i>{total}</i> }
}

pub struct Step {
    pub by: Vec<u32>,
}

fn counted(total: &u32, mut step: Step) -> u32 {
    step.by.push(1);
    total + step.by.iter().sum::<u32>()
}

/// A reducer's action, changed.
pub fn Stepped() -> JSX::Element {
    let (total, dispatch) = use_reducer(counted, 0);
    jsx! { <button onClick={move |_| dispatch.dispatch(Step { by: vec![10] })}>{*total}</button> }
}

pub struct Nudge {
    pub points: Vec<Point>,
}

fn nudged(total: &f64, mut nudge: Nudge) -> f64 {
    nudge.points[0].x += 1.0;
    total + nudge.points[0].x
}

/// A reducer's action, an item of its \`Vec\` changed.
pub fn Nudged() -> JSX::Element {
    let (total, dispatch) = use_reducer(nudged, 0.0);
    jsx! { <button onClick={move |_| dispatch.dispatch(Nudge { points: vec![Point { x: 10.0 }] })}>{*total}</button> }
}

fn started(mut seed: Step) -> u32 {
    seed.by.push(1);
    seed.by.iter().sum()
}

pub struct SeedProps {
    pub seed: Step,
}

/// An initializer's argument, changed.
pub fn Seeded(props: SeedProps) -> JSX::Element {
    let (total, _) = use_reducer_with(counted, props.seed, started);
    jsx! { <s>{*total}</s> }
}

/// A state's updater, given React's state by \`&\`: its clone, changed.
pub fn Updated() -> JSX::Element {
    let (items, set_items) = use_state(|| vec![1u32]);
    let count = items.len();
    jsx! {
        <button onClick={move |_| set_items.update(|items| {
            let mut items = items.clone();
            items.push(1);
            items
        })}>{count}</button>
    }
}

thread_local! {
    static FORWARDED: ForwardRefExoticComponent<DotProps, &'static react::webapi::Element> =
        forward_ref(|mut props: DotProps, _: ForwardedRef<&'static react::webapi::Element>| {
            props.at.x += 1.0;
            jsx! { <u>{props.at.x}</u> }
        });
}

/// A \`forward_ref\` render's props, changed.
pub fn Forwarding(props: DotProps) -> JSX::Element {
    jsx! { <FORWARDED at={props.at} /> }
}
`;

const driver = `import { expect, test } from "bun:test";
import { StrictMode, createElement } from "react";
import { flushSync } from "react-dom";
import { createRoot } from "react-dom/client";
import { Apart, Changed, Forwarding, Moved, Nudged, Passed, Seeded, Shifted, Stepped, Updated } from "./lib.jsx";

const shown = (element) => {
  const container = document.createElement("div");
  const root = createRoot(container);
  flushSync(() => root.render(createElement(StrictMode, null, element)));
  return container;
};
const clicked = (container) => {
  flushSync(() => container.querySelector("button").click());
  return container.textContent;
};

test("each renders Rust's answer under StrictMode", () => {
  const at = { x: 1 };
  expect(shown(createElement(Moved, { at })).textContent).toBe("2");
  expect(shown(createElement(Changed, { at })).textContent).toBe("2");
  expect(shown(createElement(Apart, { at })).textContent).toBe("2");
  expect(shown(createElement(Passed, { at })).textContent).toBe("2");
  expect(shown(createElement(Forwarding, { at })).textContent).toBe("2");
  expect(at).toEqual({ x: 1 });
  const items = [{ x: 1 }, { x: 2 }];
  expect(shown(createElement(Shifted, { items })).textContent).toBe("5");
  expect(items).toEqual([{ x: 1 }, { x: 2 }]);
  expect(clicked(shown(createElement(Stepped)))).toBe("11");
  expect(clicked(shown(createElement(Nudged)))).toBe("11");
  const seed = { by: [5] };
  expect(shown(createElement(Seeded, { seed })).textContent).toBe("6");
  expect(seed).toEqual({ by: [5] });
  expect(clicked(shown(createElement(Updated)))).toBe("2");
});
`;

test("what React keeps and gives again is copied where it's changed, Rust's answer under StrictMode", () => {
  const dir = fixture("react-kept");
  writeFileSync(join(dir, "lib.rs"), program);
  run([compiler, ...flags(dir)]);
  const js = readFileSync(join(dir, "lib.jsx"), "utf8");
  // A part moved out and changed, its copy where it's moved, as a clone.
  expect(js).toContain("export function Moved(props) {\n  const at = { ...props.at };\n  at.x++;\n");
  // Changed itself: its copy first.
  expect(js).toContain("export function Changed(props) {\n  props = { ...props, at: { ...props.at } };\n");
  // Taken apart where it's given: the part's copy first.
  expect(js).toContain("export function Apart({ at }) {\n  at = { ...at };\n  at.x++;\n");
  // Given on unchanged, to React: as it is.
  expect(js).toContain("export function Passed(props) {\n  return <Moved at={props.at} />;\n}");
  expect(js).toContain("  step = { ...step, by: step.by.slice() };\n  step.by.push(1);\n");
  writeFileSync(join(dir, "driver.test.jsx"), driver);
  const p = bunTest(["--preload", "./test/happydom.js", join(dir, "driver.test.jsx")]);
  const output = p.stdout + p.stderr;
  expect([p.code, output.match(/(\d+) pass/)?.[1]], output).toEqual([0, "1"]);
});

// A reducer rust-js can't see, a \`dyn Fn\` given an object it may change:
// refused, as what it changes of React's action couldn't be copied. One of
// numbers, which nothing changes in place, is as it was.
test("a reducer rust-js can't see, given an object, is refused", () => {
  const dir = fixture("react-kept-refused");
  writeFileSync(
    join(dir, "lib.rs"),
    `#![allow(non_snake_case)]
use react::{JSX, jsx, use_reducer};

pub struct Step {
    pub by: Vec<u32>,
}

pub fn Stepped() -> JSX::Element {
    let reducer: Box<dyn Fn(&u32, Step) -> u32> = Box::new(|total, mut step| {
        step.by.push(1);
        total + step.by.len() as u32
    });
    let (total, _) = use_reducer(reducer, 0);
    jsx! { <b>{*total}</b> }
}
`,
  );
  const ran = runSync([compiler, ...flags(dir)], root, 60_000);
  expect(ran.code).not.toBe(0);
  expect(ran.stderr).toContain("`use_reducer` gives this function again what it gives it, which it keeps, so what the function changes in place of it is copied first (#1723)");
  expect(ran.stderr).toContain("one rust-js can't see is #1727");
});

// A component's props of a flattened field, changed in place: its copy
// reads its object's own keys, the flattened one's among them, never a key
// of the field's name, which its object lacks (#1847): given an object
// whose \`range\` throws, as React's own lacks one.
test("a component's kept copy of flattened props reads its own keys alone", async () => {
  const dir = fixture("react-kept-flattened");
  writeFileSync(
    join(dir, "lib.rs"),
    `#![allow(non_snake_case)]
use react::{JSX, jsx};

pub struct StepRange {
    pub min: f64,
    pub step: f64,
}

pub struct Depth {
    pub at: f64,
}

pub struct RangedProps {
    #[cfg_attr(rust_js, rust_js::flatten)]
    pub range: StepRange,
    pub depth: Depth,
}

pub fn Ranged(mut props: RangedProps) -> JSX::Element {
    props.range.step += 1.0;
    props.depth.at += props.range.step + props.range.min;
    jsx! { <i>{props.depth.at}</i> }
}
`,
  );
  run([compiler, ...flags(dir)]);
  const js = readFileSync(join(dir, "lib.jsx"), "utf8");
  expect(js).toContain("props = { ...props, depth: { ...props.depth } };");
  expect(js).not.toContain("props.range");
  const lib = await import(join(dir, "lib.jsx"));
  const props = Object.create({
    get range() {
      throw new Error("a nested range read");
    },
  });
  Object.assign(props, { min: 1, step: 2, depth: { at: 0 } });
  expect(lib.Ranged(props).props.children).toBe(4);
  expect([props.step, props.depth.at]).toEqual([2, 0]);
});

// A component's `Vec` of its props, moved into a library's function that
// pushes into it: copied first, as the library's contract says its code
// changes one; one no crate changes is given as it is (#1871,
// test/deep-chain.test.ts).
test("a component's props a library changes in place are copied first", async () => {
  const dir = fixture("react-kept-library");
  const out = (crate: string) => join(dir, crate);
  const crate = (name: string, source: string) => writeFileSync(join(dir, `${name}.rs`), source);
  crate("dep", `pub fn tagged(mut names: Vec<String>) -> Vec<String> {\n    names.push("tag".to_string());\n    names\n}\n`);
  crate("app", `#![allow(non_snake_case)]\nuse react::{JSX, jsx};\n\npub struct NamesProps {\n    pub names: Vec<String>,\n}\n\npub fn Tagged(props: NamesProps) -> JSX::Element {\n    let names = dep::tagged(props.names);\n    jsx! { <i>{names.len()}</i> }\n}\n`);
  run([compiler, join(dir, "dep.rs"), "-o", join(out("dep"), "lib.js"), "--library", "--manifest", join(out("dep"), "lib.manifest.json"),
    "--", "--crate-name", "dep", `--emit=metadata=${join(out("dep"), "libdep.rmeta")}`]);
  run([compiler, join(dir, "app.rs"), "-o", join(out("app"), "lib.js"), "--dependency", join(out("dep"), "lib.manifest.json"),
    "--", "--crate-name", "app", "--extern", `dep=${join(out("dep"), "libdep.rmeta")}`, "-L", `dependency=${out("dep")}`,
    "--extern", `react=${join(target, "libreact.rmeta")}`, "-L", target]);
  const js = readFileSync(join(out("app"), "lib.jsx"), "utf8");
  expect(js).toContain("tagged(props.names.slice())");
  const lib = await import(join(out("app"), "lib.jsx"));
  const props = { names: ["a"] };
  expect([lib.Tagged(props).props.children, lib.Tagged(props).props.children, props.names]).toEqual([2, 2, ["a"]]);
});
