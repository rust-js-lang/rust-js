// A class component (rd331): a `#[rust_js::class]` struct that implements
// react's `Component` is an ES class that extends React's, as react-art's
// ReactART.js writes `class Surface extends React.Component {
// componentDidMount() { .. } render() { .. } }`. Each program is compiled
// against the react crate and rendered by real React, in happy-dom.

import { beforeAll, expect, test } from "bun:test";
import { readFileSync, writeFileSync } from "node:fs";
import { join } from "node:path";
import { runSync } from "./child";
import { buildReact, compiler, fixture, root, run, target } from "./support";

beforeAll(buildReact, 600_000);

const flags = (dir: string) => ["--declarations", join(dir, "lib.rs"), "-o", join(dir, "lib.js"), "--", "--extern", `react=${join(target, "libreact.rmeta")}`, "-L", target];

/** `rust` compiled in a fixture of `name`, against the react crate: its JS, its declarations, and where they are. */
function compiled(name: string, rust: string): { js: string; ts: string; dir: string } {
  const dir = fixture(name);
  writeFileSync(join(dir, "lib.rs"), rust);
  run([compiler, ...flags(dir)]);
  return { js: readFileSync(join(dir, "lib.jsx"), "utf8"), ts: readFileSync(join(dir, "lib.d.ts"), "utf8"), dir };
}

/** What the compiler says of `rust`, which it refuses. */
function refused(name: string, rust: string): string {
  const dir = fixture(name);
  writeFileSync(join(dir, "lib.rs"), rust);
  const ran = runSync([compiler, ...flags(dir)], root, 60_000);
  expect(ran.code).not.toBe(0);
  return ran.stderr;
}

/** `driver`, a bun test of the module in `dir`, run with a DOM: how many of its tests passed, or what it printed. */
function rendered(dir: string, driver: string) {
  writeFileSync(join(dir, "driver.test.jsx"), driver);
  const p = Bun.spawnSync(["bun", "test", "--preload", "./test/happydom.ts", join(dir, "driver.test.jsx")], { cwd: root, stderr: "pipe" });
  const output = p.stdout.toString() + p.stderr.toString();
  return { code: p.exitCode, passed: output.match(/(\d+) pass/)?.[1], output };
}

// Upstream's Surface and Text, as a class component's lifecycle has them:
// one React constructs, of fields set after; one that writes its
// constructor, `super(props)` first.
const components = `#![allow(non_snake_case)]
use std::cell::Cell;

use react::webapi::HTMLParagraphElement;
use react::{Component, JSX, jsx};

pub struct CounterProps {
    pub label: &'static str,
}

/// Says what React does with it.
#[cfg_attr(rust_js, rust_js::class)]
pub struct Counter {
    _mounts: Cell<Option<f64>>,
    _paragraph: Cell<Option<&'static HTMLParagraphElement>>,
}

impl Component<CounterProps> for Counter {
    fn component_did_mount(self: &'static Self) {
        self._mounts.set(Some(self._mounts.get().unwrap_or(0.0) + 1.0));
        println!("mount {} {}", self.props().label, self._paragraph.get().is_some());
    }

    fn component_did_update(self: &'static Self, prev_props: &CounterProps, _prev_state: &()) {
        println!("update {} to {}", prev_props.label, self.props().label);
    }

    fn component_will_unmount(self: &'static Self) {
        println!("unmount {}", self.props().label);
    }

    fn render(self: &'static Self) -> JSX::Element {
        let props = self.props();
        jsx! { <p ref={move |p| self._paragraph.set(p)}>{props.label}</p> }
    }
}

pub struct LabelProps {
    pub text: &'static str,
}

#[cfg_attr(rust_js, rust_js::class)]
pub struct Label {
    renders: Cell<f64>,
}

#[cfg_attr(rust_js, rust_js::class)]
impl Label {
    pub fn new(props: &LabelProps) -> Self {
        println!("made {}", props.text);
        Label { renders: Cell::new(0.0) }
    }

    pub fn greeting() -> &'static str {
        "hi"
    }
}

impl Component<LabelProps> for Label {
    fn component_did_mount(self: &'static Self) {
        self.force_update();
    }

    fn render(self: &'static Self) -> JSX::Element {
        self.renders.set(self.renders.get() + 1.0);
        jsx! { <span>{self.props().text}{" "}{self.renders.get()}</span> }
    }
}

/// What a label renders, called as Rust calls its method.
pub fn shown(label: &'static Label) -> JSX::Element {
    label.render()
}
`;

test("a class that implements react's Component is an ES class React renders", () => {
  const { js, ts, dir } = compiled("class-components", components);
  expect(js).toContain('import { Component } from "react";');
  // React constructs it: no constructor, its methods its impl's, each
  // where it's written; `this.props` React's.
  expect(js).toContain(`/** Says what React does with it. */
export class Counter extends Component {
  componentDidMount() {`);
  expect(js).toContain(`  componentDidUpdate(prevProps, prevState) {
    console.log(\`update \${prevProps.label} to \${this.props.label}\`);
  }`);
  // A ref callback of `self`, `&'static` as React keeps it, sets its field.
  expect(js).toContain(`        ref={(p) => {
          this._paragraph = p;
        }}`);
  // Its constructor, written, gives the class it extends its props first.
  expect(js).toContain(`export class Label extends Component {
  constructor(props) {
    super(props);
    console.log(\`made \${props.text}\`);
    this.renders = 0;
  }`);
  expect(js).toContain("    this.forceUpdate();");
  // A Rust call of its method is JS's, on the instance.
  expect(js).toContain("export function shown(label) {\n  return label.render();\n}");
  // What it doesn't write is the class it extends', declared so.
  expect(js).not.toContain("componentWillUnmount() {\n  }");
  expect(ts).toContain('import type { Component, JSX } from "react";');
  expect(ts).toContain("export declare class Counter extends Component<CounterProps, undefined> {");
  expect(ts).toContain("    componentDidUpdate(prevProps: CounterProps, prevState: undefined): void;");
  expect(js).toContain("  static greeting() {\n    return \"hi\";\n  }");
  expect(ts).toContain(
    "export declare class Label extends Component<LabelProps, undefined> {\n    constructor(props: LabelProps);\n    renders: number;\n    static greeting(): string;\n    componentDidMount(): void;\n    render(): JSX.Element;\n}",
  );
  const ran = rendered(dir, `import { expect, test } from "bun:test";
import { Component, PureComponent, createElement } from "react";
import { flushSync } from "react-dom";
import { createRoot } from "react-dom/client";
import { Counter, Label } from "./lib.jsx";

const said = [];
console.log = (line) => said.push(line);

test("React mounts, updates and unmounts it, in order", () => {
  const container = document.createElement("div");
  const root = createRoot(container);
  flushSync(() => root.render(createElement(Counter, { label: "a" })));
  expect(container.innerHTML).toBe("<p>a</p>");
  flushSync(() => root.render(createElement(Counter, { label: "b" })));
  expect(container.innerHTML).toBe("<p>b</p>");
  flushSync(() => root.unmount());
  expect(said.splice(0)).toEqual(["mount a true", "update a to b", "unmount b"]);
});

test("its constructor runs once, and forceUpdate is React's", () => {
  const container = document.createElement("div");
  const root = createRoot(container);
  flushSync(() => root.render(createElement(Label, { text: "x" })));
  expect(container.innerHTML).toBe("<span>x 2</span>");
  expect(said.splice(0)).toEqual(["made x"]);
  flushSync(() => root.unmount());
});

test("it's a real class: React's and a JS subclass's", () => {
  const counter = new Counter({ label: "z" });
  expect(counter).toBeInstanceOf(Component);
  expect(counter).not.toBeInstanceOf(PureComponent);
  expect(Counter.prototype.isReactComponent).toEqual({});
  expect(typeof counter.setState).toBe("function");
  expect(typeof counter.forceUpdate).toBe("function");
  expect(counter.props).toEqual({ label: "z" });
  expect(Object.keys(counter)).not.toContain("_mounts");
  expect(Label.greeting()).toBe("hi");
  expect(() => Counter({ label: "z" })).toThrow(TypeError);
  class Loud extends Counter {
    render() {
      return createElement("b", null, super.render());
    }
  }
  const container = document.createElement("div");
  const root = createRoot(container);
  flushSync(() => root.render(createElement(Loud, { label: "c" })));
  expect(container.innerHTML).toBe("<b><p>c</p></b>");
  flushSync(() => root.unmount());
  expect(said.splice(0)).toEqual(["mount c true", "unmount c"]);
});
`);
  expect([ran.code, ran.passed], ran.output).toEqual([0, "3"]);
}, 60_000);

test("what a class that extends one is refused says what to write", () => {
  const head = `#![allow(non_snake_case)]
use react::{Component, JSX, jsx};

pub struct P {
    pub n: f64,
}
`;
  // React constructs it: a field it sets after is `undefined` until then.
  expect(refused("class-component-field", `${head}
#[cfg_attr(rust_js, rust_js::class)]
pub struct Made {
    count: f64,
}

impl Component<P> for Made {
    fn render(self: &'static Self) -> JSX::Element {
        jsx! { <p>{self.count}</p> }
    }
}
`)).toContain("rust-js: a `#[rust_js::class]` that its superclass constructs has fields set after, `undefined` until then: `count` is an `Option`");
  // `this.props` is React's, which a method of its own would hide.
  expect(refused("class-component-props", `${head}
#[cfg_attr(rust_js, rust_js::class)]
pub struct Own {}

impl Component<P> for Own {
    fn props(self: &'static Self) -> &'static P {
        unreachable!()
    }

    fn render(self: &'static Self) -> JSX::Element {
        jsx! { <p /> }
    }
}
`)).toContain("rust-js: a `#[rust_js::class]` keeps what the class it extends has, `props`, which its impl doesn't write: React's `this.props`");
  // `super(props)`: its constructor takes what React gives it.
  expect(refused("class-component-new", `${head}
#[cfg_attr(rust_js, rust_js::class)]
pub struct Two {}

#[cfg_attr(rust_js, rust_js::class)]
impl Two {
    pub fn new(_props: &P, _more: f64) -> Self {
        Two {}
    }
}

impl Component<P> for Two {
    fn render(self: &'static Self) -> JSX::Element {
        jsx! { <p /> }
    }
}
`)).toContain("rust-js: a `#[rust_js::class]` constructor of a class that extends one takes what it's constructed with, its props, `fn new(props: P) -> Self`");
  // An instance's own property hides its prototype's.
  expect(refused("class-component-hidden", `${head}
#[cfg_attr(rust_js, rust_js::class)]
pub struct Hidden {
    render: Option<f64>,
}

impl Component<P> for Hidden {
    fn render(self: &'static Self) -> JSX::Element {
        jsx! { <p /> }
    }
}
`)).toContain("rust-js: a `#[rust_js::class]` has no method named as its field, `render`, which `this.render` would hide");
  // One class to extend.
  expect(refused("class-component-twice", `${head}
pub struct Q {}

#[cfg_attr(rust_js, rust_js::class)]
pub struct Both {}

impl Component<P> for Both {
    fn render(self: &'static Self) -> JSX::Element {
        jsx! { <p /> }
    }
}

impl Component<Q> for Both {
    fn render(self: &'static Self) -> JSX::Element {
        jsx! { <p /> }
    }
}
`)).toContain("rust-js: a `#[rust_js::class]` extends one class, a binding's it implements, `impl Component<P> for Surface`, not two");
}, 60_000);

test("a class component is in the module its impl is, which imports what it extends", () => {
  const dir = fixture("class-component-alone");
  writeFileSync(join(dir, "lib.rs"), `pub mod alone {
    use react::{Component, JSX, jsx};

    #[cfg_attr(rust_js, rust_js::class)]
    pub struct Alone {}

    impl Component for Alone {
        fn render(self: &'static Self) -> JSX::Element {
            jsx! { <hr /> }
        }
    }
}
`);
  run([compiler, ...flags(dir)]);
  expect(readFileSync(join(dir, "alone.jsx"), "utf8")).toContain(
    'import { Component } from "react";\n\nexport class Alone extends Component {\n  render() {\n    return <hr />;\n  }\n}',
  );
  // Of one that writes its constructor, where that is.
  expect(refused("class-component-apart", `pub mod apart {
    #[cfg_attr(rust_js, rust_js::class)]
    pub struct Apart {}

    #[cfg_attr(rust_js, rust_js::class)]
    impl Apart {
        pub fn new(_props: &()) -> Self {
            Apart {}
        }
    }

    pub mod elsewhere {
        use react::{Component, JSX, jsx};

        impl Component for super::Apart {
            fn render(self: &'static Self) -> JSX::Element {
                jsx! { <hr /> }
            }
        }
    }
}
`)).toContain("rust-js: a `#[rust_js::class]` implements the class it extends in the module its constructor is in, where its class is");
}, 60_000);

// rd331i: a class component is an element of `jsx!`, as JSX's `<Surface
// width={2} />` is `createElement(Surface, { width: 2 })`: its class, of its
// impl's props, which a `use` imports with it, as a component function's.
test("a class component is an element jsx! renders, its class given its props", () => {
  const dir = fixture("class-component-element");
  writeFileSync(join(dir, "lib.rs"), `pub mod shapes {
    use react::{Component, JSX, jsx};

    pub struct SurfaceProps {
        pub width: f64,
        pub title: Option<&'static str>,
    }

    #[cfg_attr(rust_js, rust_js::class)]
    pub struct Surface {}

    impl Component<SurfaceProps> for Surface {
        fn render(self: &'static Self) -> JSX::Element {
            let props = self.props();
            jsx! { <i title={props.title}>{props.width}</i> }
        }
    }

    pub struct LabelProps {
        pub text: &'static str,
    }

    #[cfg_attr(rust_js, rust_js::class)]
    pub struct Label {
        text: String,
    }

    #[cfg_attr(rust_js, rust_js::class)]
    impl Label {
        pub fn new(props: &LabelProps) -> Self {
            Label { text: props.text.to_uppercase() }
        }
    }

    impl Component<LabelProps> for Label {
        fn render(self: &'static Self) -> JSX::Element {
            jsx! { <b>{self.text.as_str()}</b> }
        }
    }

    #[cfg_attr(rust_js, rust_js::class)]
    pub struct Rule {}

    impl Component for Rule {
        fn render(self: &'static Self) -> JSX::Element {
            jsx! { <hr /> }
        }
    }
}

pub mod app {
    use react::{JSX, jsx};

    use crate::shapes::{self, Label};

    pub fn App() -> JSX::Element {
        jsx! {
            <div>
                <shapes::Surface width={2.0} />
                <shapes::Surface width={3.0} title={Some("big")} />
                <Label text="b" key="k" />
                <shapes::Rule />
            </div>
        }
    }
}
`);
  run([compiler, ...flags(dir)]);
  const app = readFileSync(join(dir, "app.jsx"), "utf8");
  expect(app).toContain('import { Label, Rule, Surface } from "./shapes.jsx";');
  expect(app).toContain(`      <Surface width={2} />
      <Surface width={3} title="big" />
      <Label text="b" key="k" />
      <Rule />`);
  const ran = rendered(dir, `import { expect, test } from "bun:test";
import { flushSync } from "react-dom";
import { createRoot } from "react-dom/client";
import { App } from "./app.jsx";

test("React constructs each class it's given", () => {
  const container = document.createElement("div");
  const root = createRoot(container);
  flushSync(() => root.render(<App />));
  expect(container.innerHTML).toBe('<div><i>2</i><i title="big">3</i><b>B</b><hr></div>');
  flushSync(() => root.unmount());
});
`);
  expect([ran.code, ran.passed], ran.output).toEqual([0, "1"]);
}, 60_000);
