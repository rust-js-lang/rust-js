import { beforeAll, expect, test } from "bun:test";
import { join } from "node:path";
import { copyFileSync } from "node:fs";
import { root, target, run, buildCompiler, buildReact, compiler } from "./support";

beforeAll(buildCompiler, 600_000);

// ADR 0041: React components, written in Rust with the react crate, are the
// JSX you'd write by hand (ADR 0040), and React runs them.
test("React components are hand-written JSX, and React runs them", () => {
  buildReact();
  const out = join(target, "react-test");
  run([compiler, "test/components.rs", "-o", join(out, "components.js"),
    "--", "--extern", `react=${join(target, "libreact.rmeta")}`, "-L", target]);
  // A module with JSX is a `.jsx` file.
  const js = require("node:fs").readFileSync(join(out, "components.jsx"), "utf8");
  expect(js).toContain("import {\n  createContext,\n  memo,\n  useContext,\n  useEffect,\n  useId,\n  useMemo,\n  useReducer,\n  useRef,\n  useState,\n} from \"react\";");
  // Props taken apart, as a component takes them; `children` as JSX children.
  expect(js).toContain("export function Card({ title, children }) {\n  return (\n    <div className=\"card\">\n      <h2>{title}</h2>\n      {children}\n    </div>\n  );\n}");
  expect(js).toContain('const [draft, setDraft] = useState("");');
  expect(js).toContain("const left = useMemo(() => todos.filter((t) => !t.done).length, [todos]);");
  // A handler stays in the JSX, of one call or of statements, as a person
  // writes it (ADR 0218).
  expect(js).toContain("onChange={(e) => setDraft(e.target.value)}\n          onKeyDown={(e) => {\n            if (e.key === \"Enter\") {\n              add();\n            }\n          }}\n        />");
  const todos = js.slice(js.indexOf("export function Todos()"), js.indexOf("export function Clock()"));
  expect(todos).not.toMatch(/const\s+\S+\s*=\s*\(?\s*</);
  expect(todos).toMatch(/<Card title="Todos">\s*<>\s*<input/);
  expect(todos).toContain("<ul>{children}</ul>");
  // A list, with its keys.
  expect(js).toContain("<li\n      key={t.id}\n      className={t.done ? \"done\" : \"\"}\n      onClick={() => dispatch({ TAG: \"Toggle\", _0: t.id })}\n    >\n      {t.text}\n    </li>");
  expect(js).toContain("<ul>{children}</ul>");
  // `Option::map` to an element: the element, or nothing.
  expect(js).toContain('{t && <p className="latest">{t.text}</p>}');
  // `()` as an effect's dependencies is `[]`, and its cleanup is a function it returns.
  expect(js).toContain("useEffect(() => {\n    setTicks((t) => (t + 10) | 0);\n    return () => {\n      setTicks(-1);\n    };\n  }, []);");
  // Components by name, as JSX tags.
  expect(js).toContain("<Todos />\n      <Clock />\n      <Themed />");
  // A context and memoized components, made once, as `const`s of the module
  // (from `thread_local!`); a provider is the context as a tag, as in React 19.
  expect(js).toContain('const Theme = createContext("light");\nconst BADGE = memo(Badge);\n/** Any two labels that aren\'t empty count as the same props. */\nconst LooseBadge = memo(Badge, (a, b) => ');
  expect(js).toContain("const theme = useContext(Theme);");
  expect(js).toContain("<BADGE label=\"outside\" />\n      <Theme value={dark ? \"dark\" : \"light\"}>");
  // `!` of a `&bool`, which rustc writes as `Not::not`.
  expect(js).toContain("setDark((d) => !d)");
  copyFileSync(join(root, "test", "react_app.jsx"), join(out, "react_app.test.jsx"));
  const p = Bun.spawnSync(["bun", "test", "--preload", "./test/happydom.ts", join(out, "react_app.test.jsx")], { cwd: root, stderr: "pipe" });
  const output = p.stdout.toString() + p.stderr.toString();
  expect([p.exitCode, output.match(/(\d+) pass/)?.[1]], output).toEqual([0, "3"]);
}, 60_000);

test("JSX preparation preserves evaluation order, conditional execution and text", async () => {
  const { fixture, compiler } = await import("./support");
  buildReact();
  const dir = fixture("jsx-semantics");
  const input = join(dir, "lib.rs");
  await Bun.write(input, `#![allow(non_snake_case)]
use react::{JSX, jsx};
#[cfg_attr(rust_js, rust_js::link_name = "globalThis.record")]
#[allow(unused_variables)]
fn record(n: i32) -> i32 {
    unreachable!()
}
pub fn Order() -> JSX::Element {
    jsx! {
        <div data-first={record(1).to_string()}>{vec![record(2), record(3), record(4)]}</div>
    }
}
pub fn ChildrenFirst() -> JSX::Element {
    let children = vec![record(1), record(2), record(3)];
    jsx! {
        <div title={record(4).to_string()}>{children}</div>
    }
}
pub fn StatementValue() -> JSX::Element {
    jsx! {
        <div title={record(1).to_string()}>{{ let n = record(2); vec![n, record(3), record(4)] }}</div>
    }
}
pub fn Conditional(flag: bool) -> JSX::Element {
    jsx! {
        <div>{if flag { vec![record(5), record(6), record(7)] } else { vec![record(8)] }}</div>
    }
}
pub fn Text() -> JSX::Element {
    jsx! {
        <div
            title="\\\"<&>\\n">
            {" leading <&>{}\\ntrailing "}
        </div>
    }
}
pub fn Capture() -> JSX::Element {
    let count = 4;
    jsx! {
        <div onClick={move |_| { record(count); record(count + 1); }} />
    }
}
pub fn Siblings(flag: bool, inner: bool) -> JSX::Element {
    let mut n = 1;
    jsx! {
        <div>
            <span title={record(1).to_string()}>{n}</span>
            {if flag { Some(jsx! {
                <b>{if inner { jsx! { <i>{record(2)}</i> } } else { jsx! { <em>{record(3)}</em> } }}</b>
            }) } else { None }}
            {{ n = record(4); n }}
        </div>
    }
}
pub struct Attrs { pub title: String }
#[cfg_attr(rust_js, rust_js::link_name = "globalThis.attributes")]
fn attributes() -> Attrs {
    unreachable!()
}
pub fn SpreadOrder() -> JSX::Element {
    let attrs = attributes();
    jsx! { <div><span {...attrs} />{{ let n = record(9); n }}</div> }
}
pub fn First() -> JSX::Element { jsx! { <b>{"first"}</b> } }
pub fn Second() -> JSX::Element { jsx! { <i>{"second"}</i> } }
pub fn ComponentOrder() -> JSX::Element {
    let mut Selected: fn() -> JSX::Element = First;
    jsx! { <div><Selected {...()} />{{ Selected = Second; record(9) }}<Selected {...()} /></div> }
}
`);
  run([compiler, input, "-o", join(dir, "lib.js"), "--", "--extern", `react=${join(target, "libreact.rmeta")}`, "-L", target]);
  const result = await import(join(dir, "lib.jsx"));
  const log: number[] = [];
  const old = globalThis.record;
  const oldAttributes = globalThis.attributes;
  globalThis.record = (n: number) => { log.push(n); return n; };
  globalThis.attributes = () => ({ get title() { log.push(8); return "snapshot"; } });
  try {
    result.Order();
    expect(log.splice(0)).toEqual([1, 2, 3, 4]);
    result.ChildrenFirst();
    expect(log.splice(0)).toEqual([1, 2, 3, 4]);
    result.StatementValue();
    expect(log.splice(0)).toEqual([1, 2, 3, 4]);
    result.Conditional(false);
    expect(log.splice(0)).toEqual([8]);
    result.Conditional(true);
    expect(log.splice(0)).toEqual([5, 6, 7]);
    expect(result.Text().props).toMatchObject({ title: '"<&>\n', children: " leading <&>{}\ntrailing " });
    const element = result.Capture();
    expect(log).toEqual([]);
    element.props.onClick();
    expect(log).toEqual([4, 5]);
    log.splice(0);
    for (const [flag, inner, expected] of [[false, false, [1, 4]], [true, false, [1, 3, 4]], [true, true, [1, 2, 4]]] as const) {
      const tree = result.Siblings(flag, inner);
      expect(log.splice(0)).toEqual([...expected]);
      expect(tree.props.children[0].props.children).toBe(1);
      expect(tree.props.children[2]).toBe(4);
    }
    const spread = result.SpreadOrder();
    expect(log.splice(0)).toEqual([8, 9]);
    expect(spread.props.children[0].props.title).toBe("snapshot");
    const components = result.ComponentOrder();
    expect(log.splice(0)).toEqual([9]);
    expect(components.props.children[0].type).toBe(result.First);
    expect(components.props.children[2].type).toBe(result.Second);
    const code = await Bun.file(join(dir, "lib.jsx")).text();
    const siblings = code.slice(code.indexOf("export function Siblings("), code.indexOf("export function SpreadOrder("));
    expect(siblings).not.toMatch(/(?:const\s+\S+|\b\w+)\s*=\s*\(?\s*</);
    const map = await Bun.file(join(dir, "lib.jsx.map")).json();
    const { decodeMappings } = await import("./sourcemap");
    expect(map.sourcesContent).toEqual([await Bun.file(input).text()]);
    expect(decodeMappings(map.mappings).length).toBeGreaterThan(10);
  } finally {
    if (old === undefined) delete globalThis.record;
    else globalThis.record = old;
    if (oldAttributes === undefined) delete globalThis.attributes;
    else globalThis.attributes = oldAttributes;
  }
});

// ADR 0043: the rest of React's and React DOM's API, run by React 19.3.
test("React's and React DOM's APIs are hand-written React, and they run", () => {
  buildReact();
  const out = join(target, "react-apis");
  run([compiler, "test/apis.rs", "-o", join(out, "apis.js"),
    "--", "--extern", `react=${join(target, "libreact.rmeta")}`, "-L", target]);
  const js = require("node:fs").readFileSync(join(out, "apis.jsx"), "utf8");
  // Built-in components are JSX tags, and `use` is `use`.
  expect(js).toContain("return (\n    <Suspense fallback={<p className=\"loading\">Loading</p>}>\n      <Greeting />\n    </Suspense>");
  expect(js).toContain("const text = use(globalThis.greeting);");
  expect(js).toContain('<Activity mode={hidden ? "hidden" : "visible"}>');
  expect(js).toContain("{[1, 2].map((n) => (\n          <Fragment\n            key={n}\n            ref={(fragment) => {");
  // Objects built by methods: a style, raw HTML, and options.
  expect(js).toContain('style={{ color: "red", fontSize: 12, "--gap": "4px" }}');
  expect(js).toContain('dangerouslySetInnerHTML={{ __html: "<i>raw</i>" }}');
  expect(js).toContain('return renderToString(<Page />, { identifierPrefix: "s-" });');
  expect(js).toContain('const root = createRoot(container, { identifierPrefix: "c-" });');
  expect(js).toContain('const LazyCard = lazy(() => import("./lazy-card.jsx"));');
  Bun.write(join(out, "lazy-card.jsx"), 'export default function LazyCard() {\n  return <em className="lazy">lazy card</em>;\n}\n');
  copyFileSync(join(root, "test", "apis.jsx"), join(out, "apis.test.jsx"));
  const p = Bun.spawnSync(["bun", "test", "--preload", "./test/happydom.ts", join(out, "apis.test.jsx")], { cwd: root, stderr: "pipe" });
  const output = p.stdout.toString() + p.stderr.toString();
  expect([p.exitCode, output.match(/(\d+) pass/)?.[1]], output).toEqual([0, "8"]);
}, 120_000);

// ADR 0573: every name is camelCase in JS, React's and Rust's own alike: a
// component's props, one spread into JSX and a hook, as React code names
// them; a function, across modules, and a data struct's and a variant's
// fields, whatever the crate. No snake_case name is left.
test("every name is camelCase in JS", async () => {
  const { fixture, compiler } = await import("./support");
  const { writeFileSync } = await import("node:fs");
  buildReact();
  const dir = fixture("react-names");
  writeFileSync(join(dir, "lib.rs"), `#![allow(non_snake_case)]
mod people;

use react::{JSX, jsx, use_state};

pub fn greet(first_name: &str) -> String {
    people::full_name(&people::make_person(first_name))
}

pub enum Shape {
    Rect { top_left: u32, bottom_right: u32 },
}

pub fn rect_width(shape: &Shape) -> u32 {
    match shape {
        Shape::Rect { top_left, bottom_right } => bottom_right - top_left,
    }
}

pub fn wide_rect() -> Shape {
    Shape::Rect {
        top_left: 1,
        bottom_right: 4,
    }
}

pub fn use_clicks() -> u32 {
    let (clicks, _) = use_state(0u32);
    *clicks
}

pub fn use_twice() -> u32 {
    use_clicks() * 2
}

pub fn use_label(n: u32) -> String {
    react::start_transition(|| ());
    format!("{n}!")
}

pub struct ButtonLook {
    pub tab_order: i32,
}

pub struct FancyButtonProps {
    pub label_text: String,
    pub on_press: Box<dyn Fn()>,
    #[cfg_attr(rust_js, rust_js::flatten)]
    pub look: ButtonLook,
}

pub fn FancyButton(FancyButtonProps { label_text, on_press, look }: FancyButtonProps) -> JSX::Element {
    jsx! {
        <button onClick={move |_| on_press()} tabIndex={look.tab_order}>{label_text}</button>
    }
}

pub struct RowAttrs {
    pub class_name: &'static str,
    pub tab_index: i32,
}

pub fn App() -> JSX::Element {
    let clicks = use_clicks();
    let attrs = RowAttrs { class_name: "row", tab_index: 2 };
    jsx! {
        <div {...attrs}>
            <FancyButton labelText={format!("{clicks} clicks")} onPress={Box::new(|| ())} look={ButtonLook { tab_order: 3 }} />
        </div>
    }
}
`);
  writeFileSync(join(dir, "people.rs"), `pub struct Person {
    pub first_name: String,
    pub last_name: String,
}

pub fn make_person(first_name: &str) -> Person {
    Person { first_name: first_name.to_string(), last_name: "Doe".to_string() }
}

pub fn full_name(person: &Person) -> String {
    format!("{} {}", person.first_name, person.last_name)
}
`);
  run([compiler, join(dir, "lib.rs"), "-o", join(dir, "lib.js"), "--", "--extern", `react=${join(target, "libreact.rmeta")}`, "-L", target]);
  const lib = await Bun.file(join(dir, "lib.jsx")).text();
  const people = await Bun.file(join(dir, "people.js")).text();
  // Rust's own: functions, across modules, and fields, of a struct and a variant.
  expect(lib).toContain('import { fullName, makePerson } from "./people.js";');
  expect(lib).toContain("export function greet(firstName) {\n  return fullName(makePerson(firstName));");
  expect(people).toContain('return { firstName, lastName: "Doe" };');
  expect(lib).toContain("export function rectWidth(shape) {\n  return (shape.bottomRight - shape.topLeft");
  // React's: a hook React finds by its name, props as React code names them,
  // and a struct spread into an element, its attributes.
  expect(lib).toContain("export function useClicks() {");
  // One calling another, and a `use_` function calling none, React's
  // `startTransition` no hook: each camelCase by the same rule.
  expect(lib).toContain("export function useTwice() {");
  expect(lib).toContain("export function useLabel(n) {");
  // No name keeps its Rust spelling.
  const snake = /\b(full_name|make_person|first_name|last_name|rect_width|wide_rect|top_left|bottom_right|use_clicks|use_twice|use_label|tab_order|label_text|on_press|class_name|tab_index)\b/;
  expect(lib).not.toMatch(snake);
  expect(people).not.toMatch(snake);
  expect(lib).toContain("export function FancyButton({ labelText, onPress, ...look }) {");
  expect(lib).toContain("<FancyButton labelText={");
  expect(lib).toContain(" onPress={");
  expect(lib).toContain('{ className: "row", tabIndex: 2 }');
  // A struct flattened into props, its fields props too.
  expect(lib).toContain("tabIndex={look.tabOrder}");
  expect(lib).toContain("tabOrder={3}");
  // `js::camel_case!`, which made a crate's every name camelCase, is gone.
  writeFileSync(join(dir, "gone.rs"), "js::camel_case!();\n");
  const gone = Bun.spawnSync([compiler, join(dir, "gone.rs"), "-o", join(dir, "gone.js"), "--", "--extern", `js=${join(target, "libjs.rmeta")}`, "-L", target]);
  expect([gone.exitCode === 0, gone.stderr.toString().includes("`js::camel_case!` is gone")], gone.stderr.toString()).toEqual([false, true]);
  const module = await import(join(dir, "lib.jsx"));
  expect(module.greet("Ada")).toBe("Ada Doe");
  expect(module.rectWidth(module.wideRect())).toBe(3);
  expect((await import(join(dir, "people.js"))).makePerson("Ada")).toEqual({ firstName: "Ada", lastName: "Doe" });
});
