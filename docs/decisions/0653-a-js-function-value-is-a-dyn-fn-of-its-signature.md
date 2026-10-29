# 0653. A JS function value is a `dyn Fn` of its signature

Status: Accepted; amended (rd192): a handler's setter takes `Option<&dyn IntoEventHandler<E>>`, and an element's `onerror` is an `EventHandler`; amended by [0700](0700-typescripts-function-gives-any-value.md):
TypeScript's `Function` is `&dyn Fn() -> Option<&'static Unknown>`; extended
by [0740](0740-a-functions-members-are-read-of-any-function.md): its members,
`js::function::name(f)`, of any function. Amends [0024](0024-web-crate.md)'s and
[0260](0260-webapi-callbacks.md)'s webapi generator, whose
function JS gives was a `&'static JsObject`, and
[0648](0648-v8s-stack-trace-api-is-the-node-crates.md), whose
`Error.captureStackTrace` is bound now, and `CallSite.getFunction()` a
`dyn Fn()`.

Case ([0262](0262-when-rust-and-js-disagree.md)): N: each is the JS
function it is, read, called and compared as JS does.

## Context

A function JS gives had two types. The js crate's, `Reflect`'s
descriptor's `get`, a state's setter (ADR 0367), node's
`Error.prepareStackTrace` (ADR 0648), were a `&'static dyn Fn` of their
signature, and `js::Kind::Function` a `&dyn Fn()`. webapi's generator
made every other one a `&'static JsObject`, "what it takes isn't said":
each event handler property, `button.onclick()`, and a queuing
strategy's `size`; node's `CallSite.getFunction()` followed it. A
`JsObject` can't be called, so a program that reads `onclick` to call it,
as React's event code reads a handler, can't; and two types for one
thing is a choice an agent gets wrong. `Error.captureStackTrace`'s
`constructorOpt?: Function` had no type, which ADR 0648 left unbound.

## Decision

The user's (2026-10-23): **a JS function value has one type, `&dyn
Fn(args) -> R` of its signature; TypeScript's `Function`, which says
none, is `&dyn Fn()`, as `js::Kind::Function` is.** Its `any` argument
is `Option<&Unknown>`, a value of any shape, and an `any` result
`Option<&'static Unknown>`.

| Binding | Was | Is |
|---|---|---|
| 766 event handler getters, `onclick` | `Option<&'static JsObject>` | `Option<&'static dyn Fn(&PointerEvent) -> Option<&'static Unknown>>` |
| of them `onbeforeunload` | the same | `.. dyn Fn(&BeforeUnloadEvent) -> Option<String>` |
| of them `onerror` | the same | `.. dyn Fn(EventOrStr<'_>, Option<&str>, Option<u32>, Option<u32>, Option<&Unknown>) -> Option<&'static Unknown>` |
| 2 `ByteLengthQueuingStrategy.size`, `CountQueuingStrategy.size` | `&'static JsObject` | `&'static dyn Fn()`, WebIDL's `Function` |
| node's `CallSite.getFunction()` | `Option<&'static JsObject>` | `Option<&'static dyn Fn()>` |
| node's `Error.captureStackTrace(target, constructorOpt?)` | unbound | `capture_stack_trace(&dyn Any, Option<&dyn Fn()>)` |

769 bindings moved, and one bound. The JS of each is what it was:
`button.onclick`, `callSite.getFunction()`.

- **A callback the generator gives is a `dyn Fn` of its WebIDL
  signature**, `&'static dyn Fn(..) -> R`, where its result was any. None
  is one yet, besides the handlers.
- **What's no function stays an object**: a callback interface,
  `walker.filter`'s `NodeFilter`, which TypeScript types as a function or
  an object of `acceptNode`; and `customElements.get(name)`'s
  `CustomElementConstructor`, a class, which `new` calls, TypeScript's
  construct signature: calling it would throw.
- **`std::ptr::eq(a, b)` of two is `a === b`**: a pointer to a `dyn Fn`
  is the JS function, as one to a JS object is the object (ADR 0285).
  Before, a `&dyn Fn` made a raw pointer and was refused, "addressable
  memory".
- **A `dyn Fn` is never falsy**, as a closure isn't (ADR 0298): whether
  a handler is set is `!!button.onclick`, as it was of the `JsObject`.
  (ADR 0298 amended: `button.onclick != null`, as of any `Option`.)
- **An event handler's setter keeps its closure**, `Option<Box<dyn
  FnMut(&E)>>`: a callback Rust gives JS to own (ADR 0260), not a value it
  reads. (Amended below, rd128: it takes what its getter gives.)
- **Held by a test, not a type**: `JsObject` is still the type of a JS
  object. `test/webapi-coverage.test.ts`, "a function the webapi crate
  reads is a dyn Fn of its signature, not a JsObject", fails on any
  property TypeScript types as a function, `((ev) => any) | null`, that
  the crate reads as a `JsObject`, by its name. It fails on the crate as
  it was, `Document.onclick` first.

## Why

- **One type, the one rustc can call**: a function read is a function
  called, kept, compared, set back.
- **TypeScript's signature, as a binding types a value**: `onclick`'s is
  `(ev: PointerEvent) => any`.
- **`Function` as `Kind::Function` has it**: the one answer the js crate
  already gave to "a function of whatever it takes".

## Alternatives

- **`JsObject` for what JS gives, `dyn Fn` for what Rust gives**: two
  types of one value, and the value can't be called.
- **`fn` pointers**: a JS function may be a closure; no binding gives
  one.
- **A `Function` type of the js crate's, with `call`**: a second way to
  call a JS function, beside `f(x)`.

## Consequences

- **TypeScript types a queuing strategy's `size` as
  `QueuingStrategySize`, `(chunk) => number`**, where the generator reads
  WebIDL's `Function`: it's `dyn Fn()`, looser in what it gives. A
  `ByteLengthQueuingStrategy`'s chunk is an `ArrayBufferView`, which a
  `dyn Fn` can't take as an `impl`.
- **`Error.captureStackTrace`'s `constructorOpt` takes a function of no
  arguments and no result**, `&dyn Fn()`: a Rust function of others isn't
  one, as TypeScript's `Function` would take it.
- A handler called through the getter, `onclick(event)`, is
  `button.onclick(event)` where the read is folded into the call, `this`
  the button; called from a variable, `this` is `undefined`, as in JS.

## What holds it

- `test/webapi-coverage.test.ts`, "a function the webapi crate reads is a
  dyn Fn of its signature, not a JsObject"; "the webapi crate is what its
  generator writes".
- `test/bindings.test.ts`, "a function JS gives is a dyn Fn of its
  signature": an `onclick` kept in a thread-local, compared by
  `std::ptr::eq`, read and called, and set to a closure, each as JS and
  run.
- `test/node-bindings.test.ts`, "V8's stack trace hook is saved, set and
  restored": `Error.captureStackTrace(target)` and
  `Error.captureStackTrace(target, above)`, run in Node, and
  `callSite.getFunction()`.
- `test/snapshots.test.ts`, `web_forms`: `!!button.onclick`, as before.
- Mutations `dyn-fn-pointer-refused`, `dyn-fn-falsy`.

## Amendment: an event handler's setter takes what its getter gives (rd128)

The user's decision (2026-10-29, React port gap rd128). A handler's setter
took a closure for JS to own, `set_onclick(Option<Box<dyn
FnMut(&PointerEvent)>>)`, and its getter gave a function of TypeScript's
signature, `Option<&'static dyn Fn(&PointerEvent) -> Option<&'static
Unknown>>`: two types of one property's value, so what was set couldn't be
compared with what's read. React's ReactDOMComponent sets `node.onclick =
noop` and later clears it only where it's still its own, `if (node.onclick
=== noop) { node.onclick = null; }`; the port could write neither, and
wrote `js::set(node, "onclick", noop as fn())`.

**A handler's setter takes the function as its getter gives it, an
`Option<&'static dyn Fn(&E)>`, `remove_event_listener`'s form** (ADR
0282): `node.set_onclick(Some(&noop))` of `fn noop(_event:
&PointerEvent) {}` is `node.onclick = noop;`, and
`node.onclick().is_some_and(|onclick| std::ptr::eq(onclick, &noop))` is
`node.onclick === noop` (ADR 0562, amended: a fn item's pointer is the
function). A closure that captures nothing is lent as it is, `Some(&|_|
work())`; one that captures is `webapi::listener(move |event| ..)`'s,
which is the closure itself.

| Handler | Setter and getter |
|---|---|
| `EventHandler`, `onclick` and 760 more | `Option<&'static dyn Fn(&E)>` |
| `onbeforeunload` | `Option<&'static dyn Fn(&BeforeUnloadEvent) -> Option<String>>` |
| `onerror` | `Option<&'static dyn Fn(EventOrStr<'_>, Option<&str>, Option<f64>, Option<f64>, Option<&Unknown>) -> R>` |

- **`onclick`'s getter gives no result**: TypeScript's `any` of `(ev) =>
  any` is what no caller reads, as a handler's result is never read but
  `onerror`'s and `onbeforeunload`'s; a JS handler's result is dropped,
  as a call of a `dyn Fn(&E)` drops it.
- **`onerror`'s setter's `R` is `bool`, its getter's `Option<&'static
  Unknown>`**: what Rust sets says whether the error is handled, `true` not
  reported; what JS set may give any value, or none, which a `bool` would
  claim it can't. Its other parameters are one type in both, `Option<&str>`
  the source, not the setter's `Option<String>`. Comparing a Rust function
  with what `onerror` holds is the one comparison left without a type; no
  program has needed it.
- **A handler that keeps its own state captures a `Cell`**, as a listener
  does: a `dyn Fn` can't change what it captures.

Case ([0262](0262-when-rust-and-js-disagree.md)): N: each is the JS
function it is, set, read and compared as JS does.

Held by `test/bindings.test.ts`, "a function JS gives is a dyn Fn of its
signature": an `onclick` set to a fn item, read back and compared, cleared
where it's still that function, kept in a thread-local, called and set to
a closure that captures, each as JS and run; `test/web_forms.rs`'s
handlers, `onerror` and `onbeforeunload` too, and its snapshot;
`test/webapi-coverage.test.ts`, "the webapi crate is what its generator
writes".

## Amendment: a handler's setter takes a union; an element's `onerror` is an `EventHandler` (rd192)

The user's decision (2026-10-29, React port gap rd192). React DOM's
`preloadStylesheet` keeps a promise on a link that its load fulfils and
its error rejects, `link._p = new Promise((resolve, reject) => {
link.onload = resolve; link.onerror = reject; })`. A setter of
`Option<&'static dyn Fn(&E)>` took neither: `resolve` of a
`Promise<&'static Event>` is a `&'static dyn Fn(&'static Event)`, another
type, and `reject` a `&'static dyn Fn(Option<&Unknown>)`; and an element's
`onerror` was typed with a window's five arguments, where the spec calls
an element's with the event alone.

**A handler's setter takes a union, `Option<&dyn IntoEventHandler<E>>`,
each member borrowed as it is (ADR 0443's form, ADR 1021's union):**

- a Rust function or closure of the event, `F: Fn(&'static E) + Copy +
  'static`, which covers the getter's `&'static dyn Fn(&E)` and a JS
  function of the kept event, `resolve`'s;
- a JS function of any value, `&'static dyn Fn(Option<&Unknown>)`,
  `reject`'s.

`Copy`, because JS keeps what Rust only lent: a closure that owns what
isn't `Copy` would be dropped as the statement ends while JS still calls
it, so it's refused, rustc's "`String: Copy` is not satisfied", and is
`webapi::listener(..)`'s, `Some(&listener(..))`, which JS owns. As a
listener's, a closure's parameter is annotated where the body reads it,
rustc's "type annotations needed" otherwise.

`onbeforeunload` takes `IntoOnBeforeUnloadEventHandler<E>` (`F:
Fn(&'static E) -> Option<String> + Copy`), and a window's `onerror`
`IntoOnErrorEventHandler` (the five arguments, giving `bool`), each with
the JS function of any value; a window's takes its getter's function too,
`Option<&'static Unknown>` its result, so it's set back as it was read.

**An element's `onerror`, and a document's, is an `EventHandler`**: the
generator gives `OnErrorEventHandler`'s five arguments only to a global
object, `[Global]`, a window (generate.ts's `HANDLERS`), `Fn(&Event)`
elsewhere, getter and setter. The getters stay as above, a window's
`onerror` giving `Option<&'static Unknown>`.

```rust
pub fn settled(link: &'static HTMLLinkElement) {
    js::set(
        link,
        "_p",
        js::promise::new(|resolve: &'static dyn Fn(&'static Event), reject| {
            link.set_onload(Some(&resolve));
            link.set_onerror(Some(&reject));
        }),
    );
}
```

```js
export function settled(link) {
  link._p = new Promise((resolve, reject) => {
    link.onload = resolve;
    link.onerror = reject;
  });
}
```

The expando is the guide's `js::set(value, key, to)`, `link._p = ..`.

Case ([0262](0262-when-rust-and-js-disagree.md)): N: each is the function
it is, set as JS sets it.

The cost: an `Option` the getter gave isn't one the setter takes, so a
handler saved and restored is `if let Some(saved) = saved {
window.set_onerror(Some(&saved)) }`.

Held by `test/webapi-events.test.ts`, "an event handler is a closure or any
JS function, each as it is, in Chromium" (a link's `onload`/`onerror` a
promise's `resolve`/`reject`, settled by dispatched events; an image's
`onerror` given the event alone; a window's given an `ErrorEvent`'s message
and line, its `true` cancelling it; a window's read and set back), and its
diagnostics (an unannotated closure, another event's, one owning a
`String`, a JS function of another argument); `test/web_forms.rs`'s
handlers and `test/compiler.test.ts`; `test/bindings.test.ts`, "a function
JS gives is a dyn Fn of its signature" (a capturing `Copy` closure set as
it is). No mutation: the change is the webapi generator's (ADR 1021).
