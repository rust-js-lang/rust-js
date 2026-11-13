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
import { buildReact, compiler, fixture, root, run, target } from "./support";

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
import { Apart, Changed, Forwarding, Moved, Passed, Seeded, Shifted, Stepped, Updated } from "./lib.jsx";

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
  const p = Bun.spawnSync(["bun", "test", "--preload", "./test/happydom.js", join(dir, "driver.test.jsx")], { cwd: root, stderr: "pipe" });
  const output = p.stdout.toString() + p.stderr.toString();
  expect([p.exitCode, output.match(/(\d+) pass/)?.[1]], output).toEqual([0, "1"]);
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
