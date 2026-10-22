// What JSX gives a tag's handlers, refs and styles: an element's events of
// their kind, async and optional handlers, a state's setter, a ref as
// @types/react's `Ref`, and styles as csstype types them. Each test is a
// program of its own, compiled and run.

import { beforeAll, expect, test } from "bun:test";
import { mkdirSync, readFileSync, writeFileSync } from "node:fs";
import { join } from "node:path";
import { createElement } from "react";
import { renderToStaticMarkup } from "react-dom/server";
import { buildReact, compiler, fixture, run, target } from "./support";

beforeAll(buildReact, 600_000);

function compile(source: string, files: Record<string, string> = {}) {
  const dir = fixture("jsx-syntax");
  for (const [name, body] of Object.entries({ "lib.rs": source, ...files })) {
    const path = join(dir, name);
    mkdirSync(join(path, ".."), { recursive: true });
    writeFileSync(path, body);
  }
  const args = [compiler, join(dir, "lib.rs"), "-o", join(dir, "lib.js"), "--manifest", join(dir, "manifest.json"),
    "--", "--extern", `react=${join(target, "libreact.rmeta")}`, "-L", target];
  return { dir, args };
}

// Keep the whole emitted module, including imports and formatting. Only the
// temporary source directory varies between runs; never normalize emitted code.

// ADR 0224: a tag's element reaches its handlers' events and its `ref`, as
// @types/react's `IntrinsicElements` gives them, while what JSX makes is an
// `JSX::Element` whatever its tag. A handler of any element's event is widened.
test("JSX gives a tag's element to its handlers and its ref", () => {
  const source = `#![deny(warnings)]
#![allow(non_snake_case)]
use react::{JSX, event, jsx, use_ref, webapi};
pub struct ButtonProps {
    pub on_click: Box<dyn Fn(&event::MouseEvent)>,
}
pub fn Button(ButtonProps { on_click }: ButtonProps) -> JSX::Element {
    jsx! { <button onClick={event::MouseEvent::widen(on_click)}>{"Go"}</button> }
}
pub fn App(busy: bool) -> JSX::Element {
    let input = use_ref(None::<&'static webapi::HTMLInputElement>);
    let label = if busy { jsx! { <span>{"…"}</span> } } else { jsx! { <b>{"Save"}</b> } };
    jsx! {
        <form>
            <input ref={input} />
            <button onClick={|e| e.current_target().set_disabled(true)}>{label}</button>
            <details onToggle={|e| e.current_target().set_open(false)} />
        </form>
    }
}
`;
  const { dir, args } = compile(source);
  run(args);
  const code = readFileSync(join(dir, "lib.jsx"), "utf8");
  expect(code).toContain('<button onClick={onClick}>Go</button>');
  expect(code).toContain("<input ref={input} />");
  expect(code).toContain("const label = busy ? <span>…</span> : <b>Save</b>;");
  expect(code).toContain("onClick={(e) => {\n          e.currentTarget.disabled = true;\n        }}");
  expect(code).toContain("e.currentTarget.open = false;");
  // An input's ref on a button, a handler of an input's event on it, and a
  // handler of any element's event not widened, are each rustc's error.
  for (const [wrong, written, error] of [
    ["<input ref={input} />", "<button ref={input} />", "the trait `react::webapi::IsA<react::webapi::HTMLInputElement>` is not implemented for `react::webapi::HTMLButtonElement`"],
    ["onClick={|e| e.current_target().set_disabled(true)}", "onClick={|e: &event::MouseEvent<webapi::HTMLInputElement>| { let _ = e; }}", "expected closure signature `for<'a> fn(&'a MouseEvent<react::webapi::HTMLButtonElement>) -> _`"],
    ["onClick={event::MouseEvent::widen(on_click)}", "onClick={on_click}", "found `dyn for<'a> std::ops::Fn(&'a react::event::MouseEvent)`"],
  ]) {
    writeFileSync(join(dir, "lib.rs"), source.replace(wrong, written));
    const result = Bun.spawnSync(args, { cwd: dir });
    expect(result.exitCode).not.toBe(0);
    expect(result.stderr.toString()).toContain(error);
  }
});

// A ref is given as a `RefObject` or a `RefCallback`, which a component's
// own prop takes, `impl IntoRef<&HTMLDivElement, M>`, and passes to its
// element as it is; one of another element is an error that says so.
test("JSX refs are @types/react's Ref and RefCallback", async () => {
  const source = `#![allow(non_snake_case)]
use react::webapi::{HTMLDivElement, HTMLElement};
use react::{IntoRef, JSX, RefCallback, RefObject, jsx};
pub fn Panel<M>(target: impl IntoRef<&'static HTMLDivElement, M>) -> JSX::Element {
    jsx! { <div ref={target} /> }
}
pub fn ByObject(target: RefObject<Option<&'static HTMLDivElement>>) -> JSX::Element {
    Panel(target)
}
pub fn ByCallback(target: RefCallback<&'static HTMLElement>) -> JSX::Element {
    jsx! { <section ref={target} /> }
}
`;
  const { dir, args } = compile(source);
  run(args);
  const jsx = readFileSync(join(dir, "lib.jsx"), "utf8");
  expect(jsx).toContain("return <div ref={target} />;");
  expect(jsx).toContain("return <section ref={target} />;");
  writeFileSync(join(dir, "lib.rs"), source.replace("RefObject<Option<&'static HTMLDivElement>>", "RefObject<Option<&'static HTMLElement>>"));
  const wrong = Bun.spawnSync(args, { cwd: dir });
  expect([wrong.exitCode === 0, wrong.stderr.toString().includes("is not a ref of `&'static react::webapi::HTMLDivElement`")], wrong.stderr.toString()).toEqual([false, true]);
});

// An async handler is the async function itself, as react.dev's
// `async function handleCopy()` is, where `spawn` in a closure would call
// one in place, `(async () => { .. })()`.
test("JSX takes an async event handler as the async function it is", async () => {
  const { dir, args } = compile(`#![allow(non_snake_case)]
use js::Promise;
use react::webapi::HTMLButtonElement;
use react::{JSX, event, jsx, use_state};
#[cfg_attr(rust_js, rust_js::link_name = "Promise.resolve")]
#[allow(unused_variables)]
fn resolved() -> Promise<()> {
    unreachable!()
}
pub struct ButtonProps {
    pub on_click: Box<dyn Fn(&event::MouseEvent<HTMLButtonElement>)>,
}
pub fn Button(ButtonProps { on_click }: ButtonProps) -> JSX::Element {
    jsx! { <button onClick={on_click}>{"Copy"}</button> }
}
pub fn Copy() -> JSX::Element {
    let (copied, set_copied) = use_state(|| false);
    let handle_copy = event::MouseEvent::spawn(async move |_| {
        resolved().await;
        set_copied.set(true);
    });
    jsx! { <div><Button onClick={handle_copy} />{if *copied { "Copied" } else { "Copy" }}</div> }
}
pub fn Parsed(text: String) -> JSX::Element {
    let (copied, set_copied) = use_state(|| false);
    let handle_copy = event::MouseEvent::spawn(async move |_| {
        let Ok(n) = text.parse::<u32>() else {
            return;
        };
        resolved().await;
        set_copied.set(n == 1);
    });
    jsx! { <div><Button onClick={handle_copy} />{if *copied { "Copied" } else { "Copy" }}</div> }
}
`);
  args.push("--extern", `js=${join(target, "libjs.rmeta")}`);
  run(args);
  const jsx = readFileSync(join(dir, "lib.jsx"), "utf8");
  expect(jsx).toContain("const handleCopy = async () => {\n    await Promise.resolve();\n    setCopied(true);\n  };");
  // One that may return early, as react.dev's does, takes no event either.
  expect(jsx.match(/const handleCopy = async \(\) => \{/g)?.length).toBe(2);
  expect(jsx).not.toContain("(async");
  const { Button } = await import(join(dir, "lib.jsx"));
  let clicked = false;
  const handler = async () => { clicked = true; };
  const button = Button({ onClick: handler });
  button.props.onClick();
  expect(clicked).toBe(true);
});

// A component's optional handler passed on to an element, as react.dev's
// Button does its onClick, is `onClick={onClick}` (ADR 0198): React
// ignores what a handler returns, and no handler does what one that calls
// nothing does.
test("JSX passes an optional event handler on as it is", async () => {
  const { dir, args } = compile(`#![allow(non_snake_case)]
use react::{JSX, ReactNode, event, jsx};
pub struct ButtonProps<C> { pub children: C, pub on_click: Option<Box<dyn Fn(&event::MouseEvent)>> }
pub fn Button<C: ReactNode>(ButtonProps { children, on_click }: ButtonProps<C>) -> JSX::Element {
    jsx! { <button onClick={move |e| if let Some(f) = &on_click { f(e.upcast()) }}>{children}</button> }
}
`);
  run(args);
  const jsx = readFileSync(join(dir, "lib.jsx"), "utf8");
  expect([jsx.includes("<button onClick={onClick}>"), jsx.includes("=>")]).toEqual([true, false]);
  const { Button } = await import(join(dir, "lib.jsx"));
  const handler = () => {};
  expect([Button({ children: "b", onClick: handler }).props.onClick === handler, Button({ children: "b" }).props.onClick]).toEqual([true, undefined]);
});

// A handler that only calls a function with nothing, `move |_| on_clear()`,
// is the function, `onClick={onClear}`, as react.dev's ClearButton has it
// (ADR 0198): React gives it the event, which a Rust function has no
// parameter for.
test("JSX passes a handler that calls a function with nothing as the function", async () => {
  const { dir, args } = compile(`#![allow(non_snake_case)]
use react::{JSX, jsx};
pub struct ClearProps { pub on_clear: Box<dyn Fn()> }
pub fn Clear(ClearProps { on_clear }: ClearProps) -> JSX::Element {
    jsx! { <button onClick={move |_| on_clear()}>{"Clear"}</button> }
}
#[cfg_attr(rust_js, rust_js::link_name = "alert")]
fn alert() {
    unreachable!()
}
pub fn Alert() -> JSX::Element {
    jsx! { <button onClick={move |_| alert()}>{"Alert"}</button> }
}
`);
  run(args);
  const jsx = readFileSync(join(dir, "lib.jsx"), "utf8");
  expect(jsx).toContain("<button onClick={onClear}>Clear</button>");
  // A binding's JS function keeps its arrow: `alert(event)` would show it.
  expect(jsx).toContain("<button onClick={() => alert()}>Alert</button>");
  const { Clear } = await import(join(dir, "lib.jsx"));
  const handler = () => {};
  expect(Clear({ onClear: handler }).props.onClick).toBe(handler);
});

// A handler prop is named as @types/react names it, `MouseEventHandler<T>`:
// `onClick?: MouseEventHandler<HTMLButtonElement>`, an `EventHandler` of
// its event, the element's, or any's by default.
test("JSX handler props are @types/react's EventHandler aliases", async () => {
  const { dir, args } = compile(`#![allow(non_snake_case)]
use react::event::{ChangeEventHandler, EventHandler, MouseEvent, MouseEventHandler};
use react::webapi::{HTMLButtonElement, HTMLInputElement};
use react::{JSX, jsx};
pub struct FieldProps {
    pub on_click: Option<MouseEventHandler<HTMLButtonElement>>,
    pub on_change: ChangeEventHandler<HTMLInputElement>,
    pub on_any: MouseEventHandler,
    pub on_same: EventHandler<MouseEvent<HTMLButtonElement>>,
}
pub fn Field(FieldProps { on_click, on_change, on_any, on_same }: FieldProps) -> JSX::Element {
    let _ = (on_any, on_same);
    jsx! {
        <>
            <button onClick={move |e| if let Some(f) = &on_click { f(e) }}>{"Go"}</button>
            <input onChange={on_change} />
        </>
    }
}
`);
  run(args);
  const jsx = readFileSync(join(dir, "lib.jsx"), "utf8");
  expect(jsx).toContain("<button onClick={onClick}>Go</button>");
  expect(jsx).toContain("<input onChange={onChange} />");
});

// A form's submit is @types/react's `SubmitEvent`, whose `submitter` is the
// button that sent it, and its native event the DOM's `SubmitEvent`; a
// `FormEvent` and an `InvalidEvent` are its names of a `SyntheticEvent`.
test("JSX onSubmit gets a SubmitEvent, of its submitter", async () => {
  const { dir, args } = compile(`#![allow(non_snake_case)]
use react::event::{FormEvent, FormEventHandler, InvalidEvent, SubmitEventHandler};
use react::webapi::HTMLFormElement;
use react::{JSX, jsx};
pub struct FormProps {
    pub on_send: fn(String),
    pub on_reset: FormEventHandler<HTMLFormElement>,
    pub on_submitted: Option<SubmitEventHandler<HTMLFormElement>>,
}
pub fn checked(e: &InvalidEvent, f: &FormEvent) -> bool {
    e.is_default_prevented() || f.is_default_prevented()
}
pub fn Form(FormProps { on_send, on_reset, on_submitted }: FormProps) -> JSX::Element {
    let _ = on_submitted;
    jsx! {
        <form
            onSubmit={move |e| {
                e.prevent_default();
                let by = e.submitter().map(|b| b.title()).unwrap_or_default();
                on_send(format!("{by} {}", e.native_event().submitter().is_some()));
            }}
            onReset={on_reset} />
    }
}
`);
  run(args);
  const { Form } = await import(join(dir, "lib.jsx"));
  const sent: string[] = [];
  const { props } = Form({ onSend: (s: string) => sent.push(s), onReset: () => {} });
  const button = { title: "Send" };
  props.onSubmit({ preventDefault() {}, submitter: button, nativeEvent: { submitter: button } });
  props.onSubmit({ preventDefault() {}, submitter: null, nativeEvent: { submitter: null } });
  expect(sent).toEqual(["Send true", " false"]);
});

// `onInput` is @types/react's `InputEventHandler`, of an `InputEvent`, as
// `onBeforeInput` is; `onChange` alone is a `ChangeEvent`, of `value()`.
test("JSX onInput gets an InputEvent, as @types/react types it", async () => {
  const { dir, args } = compile(`#![allow(non_snake_case)]
use react::event::InputEventHandler;
use react::webapi::HTMLInputElement;
use react::{JSX, jsx};
pub struct FieldProps {
    pub on_read: fn(String),
    pub on_typed: Option<InputEventHandler<HTMLInputElement>>,
}
pub fn Field(FieldProps { on_read, on_typed }: FieldProps) -> JSX::Element {
    let _ = on_typed;
    jsx! {
        <input
            onInput={move |e| on_read(format!(
                "{} {} {}",
                e.data().unwrap_or_default(),
                e.native_event().input_type(),
                e.current_target().value(),
            ))}
            onChange={move |e| on_read(e.value())} />
    }
}
`);
  run(args);
  const { Field } = await import(join(dir, "lib.jsx"));
  const read: string[] = [];
  const { props } = Field({ onRead: (s: string) => read.push(s) });
  props.onInput({ data: "a", nativeEvent: { inputType: "insertText" }, currentTarget: { value: "ba" } });
  props.onChange({ target: { value: "ba" } });
  expect(read).toEqual(["a insertText ba", "ba"]);
});

// An event's `native_event` is the DOM's event it wraps, typed as
// @types/react types `nativeEvent`: a click's a `MouseEvent`, a pointer's a
// `PointerEvent`, a wheel's a `WheelEvent`, so its own fields are read.
test("JSX native_event is the DOM event of its kind", async () => {
  const { dir, args } = compile(`#![allow(non_snake_case)]
use react::{JSX, jsx};
pub struct PadProps { pub on_read: fn(String) }
pub fn Pad(PadProps { on_read }: PadProps) -> JSX::Element {
    jsx! {
        <div
            onClick={move |e| on_read(e.native_event().client_x().to_string())}
            onPointerDown={move |e| on_read(e.native_event().pointer_type())}
            onWheel={move |e| on_read(e.native_event().delta_y().to_string())} />
    }
}
`);
  run(args);
  const jsx = readFileSync(join(dir, "lib.jsx"), "utf8");
  expect(jsx).toContain("e.nativeEvent.clientX");
  const { Pad } = await import(join(dir, "lib.jsx"));
  const read: string[] = [];
  const { props } = Pad({ onRead: (s: string) => read.push(s) });
  props.onClick({ nativeEvent: { clientX: 3 } });
  props.onPointerDown({ nativeEvent: { pointerType: "pen" } });
  props.onWheel({ nativeEvent: { deltaY: 1.5 } });
  expect(read).toEqual(["3", "pen", "1.5"]);
});

// As react.dev's Button's `style={style}` of an optional prop: `None` none.
test("JSX passes an optional style on as it is", async () => {
  const { dir, args } = compile(`#![allow(non_snake_case)]
use react::{JSX, CSSProperties, jsx};
pub struct BoxProps { pub style: Option<CSSProperties> }
pub fn Panel(BoxProps { style }: BoxProps) -> JSX::Element {
    jsx! { <div style={style} /> }
}
`);
  run(args);
  expect(readFileSync(join(dir, "lib.jsx"), "utf8")).toContain("<div style={style} />");
  const { Panel } = await import(join(dir, "lib.jsx"));
  expect([renderToStaticMarkup(Panel({ style: { color: "red" } })), renderToStaticMarkup(Panel({}))]).toEqual(['<div style="color:red"></div>', "<div></div>"]);
});

// A CSS property takes what @types/react's `CSSProperties`, csstype's
// `Properties<string | number>`, types it as: `width` a length, a number in
// pixels or text, `opacity` and `z_index` a number, `color` text only.
test("JSX styles take the values csstype types them as", async () => {
  const { dir, args } = compile(`use react::{CSSProperties, JSX, jsx};
pub fn Panel() -> JSX::Element {
    jsx! { <div style={CSSProperties::new().width(300).opacity(0.5).z_index(2).color("red").margin_top("1em")} /> }
}
`);
  run(args);
  const { Panel } = await import(join(dir, "lib.jsx"));
  expect(renderToStaticMarkup(Panel())).toBe('<div style="width:300px;opacity:0.5;z-index:2;color:red;margin-top:1em"></div>');
  const refused = compile(`use react::{CSSProperties, JSX, jsx};\npub fn Panel() -> JSX::Element {\n    jsx! { <div style={CSSProperties::new().color(3)} /> }\n}\n`);
  const failed = Bun.spawnSync(refused.args, { cwd: refused.dir });
  expect([failed.exitCode === 0, failed.stderr.toString().includes("is not text")], failed.stderr.toString()).toEqual([false, true]);
});

// A style spreads another over its own, as react.dev's console box writes
// `style={{width, height, ...customStyles}}`; `None` spreads nothing.
test("JSX styles spread another style over their own", async () => {
  const { dir, args } = compile(`#![allow(non_snake_case)]
use react::{JSX, CSSProperties, jsx};
pub struct BoxProps<'a> { pub width: &'a str, pub custom_styles: Option<CSSProperties> }
pub fn Panel(BoxProps { width, custom_styles }: BoxProps) -> JSX::Element {
    jsx! { <div style={CSSProperties::new().width(width).spread(custom_styles)} /> }
}
`);
  run(args);
  expect(readFileSync(join(dir, "lib.jsx"), "utf8")).toContain("<div style={{ width, ...customStyles }} />");
  const { Panel } = await import(join(dir, "lib.jsx"));
  expect([renderToStaticMarkup(Panel({ width: "6px", customStyles: { width: "7px", color: "red" } })), renderToStaticMarkup(Panel({ width: "6px" }))])
    .toEqual(['<div style="width:7px;color:red"></div>', '<div style="width:6px"></div>']);
});

// A state's setter is a function, as @types/react's `Dispatch` is: given
// where a prop wants one, as react.dev's home page gives `setSearchText`,
// and `setHoverLine` to a memo'd CodeBlock, whose props are `'static`.
test("a state's setter is given as the function it is", async () => {
  const { dir, args } = compile(`#![allow(non_snake_case)]
use react::{JSX, jsx, use_state};
pub struct InputProps<'a> { pub value: &'a str, pub on_change: &'a dyn Fn(String), pub on_clear: Option<&'static dyn Fn(String)> }
pub fn Input(InputProps { value, on_change, on_clear }: InputProps) -> JSX::Element {
    let _ = (on_change, on_clear);
    jsx! { <b>{value}</b> }
}
pub fn Search() -> JSX::Element {
    let (text, set_text) = use_state(|| String::from("a"));
    jsx! { <Input value={text.as_str()} onChange={&**set_text} onClear={Some(&**set_text)} /> }
}
`);
  run(args);
  expect(readFileSync(join(dir, "lib.jsx"), "utf8")).toContain("<Input value={text} onChange={setText} onClear={setText} />");
  const { Search } = await import(join(dir, "lib.jsx"));
  expect(renderToStaticMarkup(createElement(Search))).toBe("<b>a</b>");
});

// A style kept, in state say, is given by reference as it is, as react.dev's
// home page keeps `{ width, height, transform }`s for its hover overlays.
test("JSX styles are given by reference", async () => {
  const { dir, args } = compile(`#![allow(non_snake_case)]
use react::{JSX, CSSProperties, jsx};
pub struct OverlaysProps<'a> { pub styles: &'a [CSSProperties], pub first: Option<&'a CSSProperties> }
pub fn Overlays(OverlaysProps { styles, first }: OverlaysProps) -> JSX::Element {
    jsx! {
        <>
            <i style={first} />
            {styles.iter().map(|styles| jsx! { <b style={styles} /> }).collect::<Vec<_>>()}
        </>
    }
}
`);
  run(args);
  const jsx = readFileSync(join(dir, "lib.jsx"), "utf8");
  expect(jsx).toContain("<b style={styles} />");
  expect(jsx).toContain("<i style={first} />");
  const { Overlays } = await import(join(dir, "lib.jsx"));
  expect(renderToStaticMarkup(Overlays({ styles: [{ width: "1px" }, { width: "2px" }], first: { color: "red" } })))
    .toBe('<i style="color:red"></i><b style="width:1px"></b><b style="width:2px"></b>');
});

// ADR 0040: a component's handler of one call whose JS gives `undefined`,
// held by MIR ahead of a borrow's call, is `() => f()` where it's read in
// place, as react.dev's ExpandableExample toggles.
test("a held handler of one call is a concise arrow", async () => {
  const { dir, args } = compile(`#![allow(non_snake_case)]
use react::webapi::MouseEvent;
use react::{JSX, ReactNode, jsx, use_state};
fn cn(parts: &[Option<&str>]) -> String {
    parts.iter().flatten().copied().collect::<Vec<_>>().join(" ")
}
pub struct ButtonProps<'a, C: ReactNode> {
    pub children: C,
    pub on_click: Option<Box<dyn Fn(&MouseEvent)>>,
    pub class_name: Option<&'a str>,
}
pub fn Button<C: ReactNode>(ButtonProps { children, on_click, class_name }: ButtonProps<C>) -> JSX::Element {
    let _ = on_click;
    jsx! { <button className={class_name.unwrap_or_default()}>{children}</button> }
}
pub fn Example(deep: bool) -> JSX::Element {
    let (expanded, set_expanded) = use_state(|| false);
    jsx! {
        <Button
            className={Some(cn(&[deep.then_some("deep"), (!deep).then_some("plain")]).as_str())}
            onClick={Some(Box::new(move |_| set_expanded.update(|current| !current)))}>
            <span>{if *expanded { "Hide" } else { "Show" }}</span>
        </Button>
    }
}
`);
  run(args);
  const jsx = readFileSync(join(dir, "lib.jsx"), "utf8");
  expect(jsx).toContain("onClick={() => setExpanded((current) => !current)}");
  const lib = await import(join(dir, "lib.jsx"));
  expect(renderToStaticMarkup(createElement(() => lib.Example(true)))).toBe('<button class="deep"><span>Show</span></button>');
});
