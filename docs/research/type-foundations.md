# Type foundations: TypeScript, Scala.js, ReScript and rust-js

Research date: 2026-10-06. This is a source review of local checkouts. It
describes what each project's code and declarations say at the revisions
below, not what their latest releases do. Citations are `project/path:line`
within each repository, except where noted.

| Project | Inspected revision | Commit date |
| --- | --- | --- |
| TypeScript (the Go port, `tsc/`) | `1f70213d4922b434345f639b441681e470c7cfc1` | 2026-09-04 |
| DefinitelyTyped (`types/react`, read with `git show`) | `fecf6d8fcc1f992c6f99f8991463bca951a73d3e` | 2026-09-30 |
| Scala.js | `5cc1be6722317e2ae6d0fea6d9d4066f27239eae` | 2026-09-15 |
| scala-js-dom | `1da1115ee18bbbc1239f224dec3cb23df6222852` | 2025-07-23 |
| Laminar | `1210af000e5695b461fb0d3fb52df2591c3ded4b` | 2026-09-24 |
| scala-js-ts-importer | `c4da3d160b680d37f710ae27b5c4cc5beb21a8a1` | 2025-08-01 |
| ReScript (13.0.0-alpha.7) | `5b00bcf69a8946aaf608bfe9c111f95c55aaffc1` | 2026-09-24 |
| @rescript/webapi (0.1.0) | `b46bc2c08a500ff28f42e91f658124906c740773` | 2026-09-19 |
| rescript-react (0.16.0-beta.1) | `bb34644ca803e021e7218a8a63b572705df73701` | 2026-07-31 |
| rescript-lang.org (docs) | `9b534e2f8ca631d83e09efb94194007e1f820bde` | 2026-09-21 |
| rust-js | `3fd82d69af14de6b6dc203591ed81403b2be0d35` | 2026-10-06 |

## Conclusion

Every ecosystem types JavaScript the same way: in layers, each a file of
declarations that names the layer below and adds to it, never copying it.

```
            TypeScript          Scala.js            ReScript               rust-js
 framework  @types/react        Laminar             rescript-react         react crate
 DOM        lib.dom.d.ts        scala-js-dom        @rescript/webapi       webapi crate
 JS std     lib.es5, es2015…    js.Promise, Array…  Stdlib                 js crate
 bindings   declare …           @js.native, @JSImport…  @module @send @get…  extern + link_name
 language   checker intrinsics  Scala types         predef types           Rust types
```

None of them types everything; each leaves some join loose. So "on par"
means the best of the four at each join, not any one of them whole.

rust-js is level with or ahead of them at the bottom: its values are the
JS a person writes, as ReScript's are; its DOM is generated from WebIDL, as
TypeScript's is, where scala-js-dom is hand-written and @rescript/webapi was
generated once and then edited; and inheritance is `Deref`, so upcasts are
free. It falls behind where a specific type should travel up a layer:

1. A tag's element type doesn't reach its handlers and its `ref`: every
   intrinsic tag is the same `react::Element`, and an event's
   `current_target` is `&webapi::Element`. TypeScript and Laminar keep it.
2. Event names aren't tied to event types in `webapi`, nor tag names to
   element types. TypeScript has both maps, from `@webref/events` and
   `@webref/elements`.
3. The `.d.ts` rust-js writes widens much to `any`: enums with fields,
   closures, every non-local type without `#[rust_js::types]`. ReScript's
   genType writes enums with fields as tagged unions and closures with their
   parameters.
4. There's no typed value of unknown shape, as ReScript's `unknown` and
   `JSON.t`, so `webapi` skips what returns one, `Response.json()`.

## Layer 0: the language's own types

**TypeScript.** The checker makes the intrinsic types itself
(`tsc/internal/checker/checker.go:983-1023`): `any` and its hidden variants
(`autoType`, `wildcardType`, `errorType`, `unresolvedType`), `unknown`,
`undefined`, `null`, `string`, `number`, `bigint`, `symbol`, `void`, `never`
and `object` (`nonPrimitiveType`). `boolean` isn't one; it's the union of
`true` and `false` (`checker.go:1010`). A primitive's methods come from the
lib: `getApparentType` maps a string to `interface String`
(`checker.go`, about line 22085).

**Scala.js.** Scala's types, with `js.Any` a third root beside `AnyRef` and
`AnyVal` (`scala-js/library/src/main/scala-new-collections/scala/scalajs/js/Any.scala:60`).
`js.|` is an encoding, a sealed trait with implicit `Evidence`, not a union
the checker knows (`library/src/main/scala/scala/scalajs/js/Union.scala`);
`js.UndefOr[A]` is `A | Unit` (`js/package.scala:85`).

**ReScript.** The compiler's predefined types (`rescript/compiler/ml/predef.ml:32-68`).
Most are the JS value itself (`rescript-lang.org/apps/docs/markdown-pages/docs/manual/shared-data-types.mdx`):
a record is an object, a tuple an array, `unit` is `undefined`. An `option`
is the value or `undefined`, with a nested `Some(None)` boxed
(`rescript/packages/@rescript/runtime/Primitive_option.res:9`); a variant
with a payload is `{TAG, _0}`, one without a string; an `@unboxed` variant
is its payload, told apart by `typeof` and `instanceof` (`docs/manual/variant.mdx:360-411`).
`int` is 32 bits.

**rust-js.** The same choices as ReScript's, made in its ADRs: `Option`
(0030, 0051, with the same nested-`None` box), enums with fields as
ReScript's tagged objects (0033), untagged enums (0214), 32-bit integers
wrapped and 64-bit ones `BigInt` (0011, 0086), a JS object type as a struct
of a `JsObject` (0111). `docs/semantics.md` summarizes them.

## Layer 1: the JS standard library, and the escape hatch

**TypeScript.** Hand-written declaration files, one per ECMAScript version,
reopened by declaration merging (`tsc/internal/bundled/libs/lib.es5.d.ts`:
`Promise<T>` at 1548, `Array<T>` at 1323; `lib.es2015.iterable.d.ts:74`
reopens `Array<T>`). `target` and `lib` pick them
(`tsc/internal/tsoptions/enummaps.go`, `targetToLibMap` at 207). The escape
hatch is `any`; the safe one `unknown`.

**Scala.js.** `js.Promise`, `js.Array`, `js.Function0..22`, `js.Iterable`,
`js.Symbol`, `js.BigInt` (`library/src/main/scala/scala/scalajs/js/`). The
escape hatch is `js.Dynamic`, whose every member is another `js.Dynamic`
(`js/Dynamic.scala:31`).

**ReScript.** `Stdlib` (`rescript/packages/@rescript/runtime/Stdlib.res:3-63`):
`Promise`, `Array`, `Dict`, `Null`, `Nullable`, `JsExn`, `JSON`, `RegExp`,
typed arrays. A value of unknown shape has a type: `unknown`
(`predef.ml:338-357`), and `Type.Classify.classify` turns it into a variant
by `typeof` (`Stdlib_Type.resi:30-76`). JSON is an `@unboxed` recursive
variant over the parsed value (`Stdlib_JSON.resi:8-16`). The escape hatches
are `Obj.magic` (`Obj.res`), an external of `"%identity"`, and `%raw`.
Typed arrays share one type per element kind, so a `Uint8Array.t` is an
`Int32Array.t` (`Stdlib_Uint8Array.res`).

**rust-js.** The `js` crate (`builtins/src/lib.rs`, ADR 0102): `JsObject`,
`Promise<T>`, `JsError`, `ArrayBuffer`, `Uint8Array`, `RegExp`, timers,
`object::from_entries`. There's no general unknown value and no dynamic
property access. A binding is trusted as declared (`docs/semantics.md`),
`link_name = "this"` is an unchecked cast, and `&'static JsObject` is an
opaque object.

## The binding toolkit

**Scala.js.** `@js.native`, `@JSGlobal`, `@JSImport`, `@JSName`,
`@JSBracketAccess`, `@JSExportTopLevel` (`library/src/main/scala/scala/scalajs/js/annotation/`).

**ReScript.** The richest of the three (`docs/manual/interop-cheatsheet.mdx:13-47`;
parsed in `rescript/compiler/frontend/ast_external_process.ml:345-388`):
`@module`, `@val`, `@scope`, `@send`, `@get`/`@set`, `@get_index`/`@set_index`
(dynamic keys), `@new`, `@variadic`, `@unwrap`, `@string`/`@int` (a
polymorphic variant argument as its literal), `@as`, `@ignore`,
`@return(nullable)`, `@this` (a callback given JS `this`), `@meth`, `@tag`,
`@unboxed`, `@notUndefined`, optional record fields. The docs show two ways
to tie an event name to its callback type, a `@string` polymorphic variant
whose payload is the callback (`bind-to-js-function.mdx:326-364`) and a GADT
(`generalized-algebraic-data-types.mdx:210-300`), but neither the DOM nor
the React bindings use them.

**rust-js.** `extern "Rust"` with `link_name` forms (ADRs 0021, 0024, 0028,
0039, 0040): a global path, `module#name`, `get x`, `set x`, `new X`,
`instanceof C`, `this`, `this()`, the JSX forms; and tool attributes:
`#[rust_js::name]`, `types`, `nullable`, `default`, `flatten`, `untagged`
and `otherwise`, `variadic`, `camel_case` (ADRs 0039, 0196, 0204, 0212,
0214, 0221, 0046). Compared with ReScript it has no `@this` callback and no
dynamic-key access; its fieldless enums are already string literals, as
`@string` gives.

## Layer 2: the DOM

| | TypeScript `lib.dom.d.ts` | scala-js-dom | @rescript/webapi | rust-js `webapi` |
| --- | --- | --- | --- | --- |
| Source | WebIDL, generated (TypeScript-DOM-lib-generator) | hand-written | WebIDL once, by a fork of TypeScript's generator, then edited by hand | WebIDL, generated |
| Coverage | the specs it's generated from | 636 files, kept by hand | 337 modules, opted into by feature | 74 chosen interfaces; members with types it can't take are skipped and counted |
| Inheritance | `extends` | `abstract class … extends` | nominal records; methods copied down by functors; upcasts by `as*` calls | `Deref`, so upcasts are automatic |
| Event name → event type | yes: `HTMLElementEventMap` | no: `addEventListener[T <: Event](type: String, …)` | no: the caller picks `'event` | no: `type_: &str`, the listener takes `&Event` |
| Tag name → element type | yes: `HTMLElementTagNameMap` | no: `createElement(String): Element` | no | no |
| Checked downcast | `instanceof` | pattern match | `classify` (`instanceof` and `Obj.magic`) | none generated; `unchecked_from` |
| WebIDL enums | string-literal unions | `String` | polymorphic variants | `&str` |

**TypeScript.** The DOM files aren't edited by hand
(`tsc/internal/bundled/README.md`). The event maps tie each name to its
event (`lib.dom.d.ts`: `GlobalEventHandlersEventMap` at 16653, where `"click"`
is a `PointerEvent`; `HTMLElementEventMap` at 17721; `DocumentEventMap` at
12539; `WindowEventMap` at 41157), and every element interface repeats the
typed overload with `this` set to it, beside a fallback for any string
(about line 17401):

```ts
addEventListener<K extends keyof HTMLElementEventMap>(type: K, listener: (this: HTMLButtonElement, ev: HTMLElementEventMap[K]) => any, options?: boolean | AddEventListenerOptions): void;
addEventListener(type: string, listener: EventListenerOrEventListenerObject, options?: boolean | AddEventListenerOptions): void;
```

`HTMLElementTagNameMap` (43099) types `createElement` and `querySelector`
by tag. `Event.target` and `currentTarget` are `EventTarget | null`, with no
element type (14189, 14227).

The maps come from data WebIDL doesn't have. In the generator, as forked in
`experimental-rescript-webapi/tools/TypeScript-DOM-lib-generator/`,
`src/build/webref/events.ts` reads `@webref/events` (each event's type, its
interface, its targets and its bubbling path) and
`src/build/webref/elements.ts` reads `@webref/elements` (each element's
interface). Both are webref packages, as rust-js's `@webref/idl` is.

**scala-js-dom.** "A hodgepodge of auto-generated/scraped/hand-tweaked code"
(`readme/Index.scalatex:158`); no generator in the build.
`addEventListener[T <: Event](type: String, listener: js.Function1[T, _], …)`
lets the caller pick `T` for any name (`EventTarget.scala`, about 25-48);
`html.Button` is an alias of `HTMLButtonElement` (`html.scala`).

**@rescript/webapi.** Generated once, then edited:
"The generation does no longer happen automatically, so manual improvements
will not be overwritten" (`docs/content/docs/contributing/code-generation.mdx:7-51`).
Inheritance is a record spread for the data and a functor chain for the
methods (`src/html/HTMLButtonElement.res`; `src/html/HTMLElement.res:19-26`).
`addEventListener: (T.t, EventType.t, DOM.EventListener.t<'event>, …)`
leaves `'event` free (`src/event/EventTarget.res:28-56`): its test annotates
a mouse event, and another passes a callback of no argument.

**rust-js.** `webapi/generate.ts` writes `webapi/src/lib.rs` from
`@webref/idl` 3.84.0 (ADR 0024): one struct per interface, `Deref` to its
parent, its members free functions in a module of its name. The only
callback type is `EventListener`, as `Box<dyn FnMut(&Event)>`:

```rust
pub safe fn add_event_listener(this: &EventTarget, type_: &str, callback: Box<dyn FnMut(&Event)>);
```

Unions are untagged enums (ADR 0215), sequences slices (ADR 0219). Skipped:
unions in results, callbacks other than `EventListener` (no
`requestAnimationFrame`, no `on*` attributes), `any` (no `Response.json()`),
interfaces outside the list, overload clashes.

## Layer 3: React

| | @types/react | Laminar | rescript-react | rust-js `react` |
| --- | --- | --- | --- | --- |
| Tag → element type | `IntrinsicElements.button` is `DetailedHTMLProps<ButtonHTMLAttributes<HTMLButtonElement>, HTMLButtonElement>` | `button: HtmlTag[dom.HTMLButtonElement]` | no: lowercase tags take one `domProps` | no: every tag is `Element` |
| Event kind per handler | yes: `onClick?: MouseEventHandler<T>` | yes: `onClick: EventProp[dom.MouseEvent]` | yes: `onClick?: JsxEvent.Mouse.t => unit` | yes: `on_click(impl Fn(&event::Mouse))` |
| Handler knows its element | yes: `currentTarget: EventTarget & T` | through `inContext` or the element's `ref` | no: `currentTarget` is an open object `{..}` | no: `&webapi::Element` |
| `ref` knows its element | yes: `Ref<T>` | yes: `ref: dom.html.Button` | no: `Dom.element` | no: its type is free |
| Attributes per tag | yes: `ButtonHTMLAttributes<T>` | not researched | no: one record for every tag | no: one set of methods for every tag |
| Built on the DOM layer | yes: `type NativeMouseEvent = MouseEvent` | yes: scala-js-dom types | no: its own minimal `Dom.res` | yes: `webapi` |

**@types/react** (`types/react/index.d.ts` at `fecf6d8`):
`SyntheticEvent<T = Element, E = Event> extends BaseSyntheticEvent<E, EventTarget & T, EventTarget>`
(2140), so `currentTarget` is the element and `target` any `EventTarget`;
`MouseEventHandler<T> = EventHandler<MouseEvent<T>>` (2335);
`DOMAttributes<T>` gives each handler `T` (2360); `IntrinsicElements`
gives each tag its element (4279 for `button`). It names the DOM's types,
`type NativeMouseEvent = MouseEvent` (9-23), and stubs them in `global.d.ts`
when there's no DOM lib. Its loose joins: `BaseSyntheticEvent`'s `any`
defaults (2115), handlers made bivariant by `bivarianceHack` (2316), a
`ChangeEvent.target` kept wrong for compatibility (2190), and hand-written
event props that disagree with the DOM's maps (`onClick` is a `MouseEvent`,
lib.dom's `"click"` a `PointerEvent`).

**Laminar.** Generated from Scala DOM Types' curated data, not WebIDL
(`project/DomDefsGenerator.scala`). Tags carry their element,
`lazy val button: HtmlTag[dom.HTMLButtonElement] = htmlTag("button")`
(`src/main/scala/com/raquo/laminar/defs/tags/HtmlTags.scala:512`), into
`ReactiveHtmlElement[+Ref <: dom.html.Element]`, whose `ref` is that type
(`nodes/ReactiveHtmlElement.scala:13`). Events carry their kind,
`lazy val onClick: EventProp[dom.MouseEvent] = eventProp("click")`
(`defs/eventProps/GlobalEventProps.scala:34`), but not the element:
`mapToValue` casts `ev.target` (`keys/EventProcessor.scala:91`), and any
event binds on any element (`modifiers/EventListener.scala:12`).

**rescript-react.** Event modules over a phantom tag
(`rescript/packages/@rescript/runtime/JsxEvent.res:7`), whose `target`,
`currentTarget` and `nativeEvent` are open objects:

```
@get external currentTarget: Type.t => {..} = "currentTarget"  /* Should return Dom.eventTarget */
```

(`JsxEvent.res:16-33`). One `domProps` record serves every tag
(`JsxDOM.res:21-24`); a `ref` holds a `Dom.element` (`rescript-react/src/ReactDOM.res:81-89`).
It doesn't use @rescript/webapi.

**rust-js.** `react/src/elements.rs`: `#[link_name = "<button>"] pub safe fn button() -> Element;`
(3580), and every attribute and handler a method of `Element`, about 1,300
of them, with `on_click(self, handler: impl Fn(&event::Mouse) + 'static)`.
Events derive from one another by `Deref` (`react/src/event.rs`), with

```rust
current_target: &'static webapi::Element = "currentTarget";
native_event: &'static webapi::Event = "nativeEvent";
```

and a DOM ref `pub fn r#ref<H, M>(self, value: impl RefValue<H, M>)`
(`react/src/lib.rs:255`), its `H` free. Per-element attribute structs exist
(`react/src/attributes.rs`, ADR 0208), generated from @types/react, but
only for a component's props to flatten; the element type is a string in
their `#[rust_js::types]`.

## Types out to TypeScript

**Scala.js** writes none: the linker's output is JS and its source map
(`linker-interface/shared/src/main/scala/org/scalajs/linker/interface/OutputPatterns.scala`).

**ReScript's genType** writes a `.gen.tsx` beside each module
(`docs/manual/typescript-integration.mdx`; `rescript/compiler/gentype/`):
records as `readonly` objects, variants as their literal and `{TAG}` unions,
`option` as `undefined | a`, `Null.t` as `null | a`, `JSON.t` as `unknown`,
functions with their parameters, an abstract type as a branded class
(`emit_type.ml:363-374`). Every value is cast, `XJS.f as any`, so nothing
checks the claim.

**rust-js** writes a `.d.ts` per module (ADRs 0196, 0207, 0210;
`src/lower/declarations.rs`). An enum with fields is `any` (274), though its
JS is the same `{TAG}` shape genType declares; a closure, a function pointer
and a `dyn` value are `(...args: any[]) => any` (329-340), and a `dyn Trait`
isn't callable in JS (ADR 0049); every non-local type without
`#[rust_js::types]` is `any` (389-391): `HashMap`, `Result`, `Rc`,
`js::Promise`, every `webapi` type; an `async fn`'s result is `any` (413).
Inherent methods, trait dictionaries and bindings aren't declared.

## Types in from TypeScript

**Scala.js** has ScalablyTyped and the older ts-importer, which says of
itself "The process is not 100 % accurate, so manual editing is often
needed" (`scala-js-ts-importer-mill/README.md`): `keyof X` is `String`,
`T[K]` is `js.Any`, a literal union its base type
(`tsimporter/src/tsimporter/Importer.scala:258-393`).

**ReScript** reads none; bindings are written by hand.

**rust-js** reads TypeScript with TypeScript's own parser
(`@rust-js/typescript`, ADR 0206), so far only to generate React's
attribute structs (`react/attributes.ts`). Generating bindings from `.d.ts`
was rejected; a checker of a binding against its library's `.d.ts` is
designed but deferred (ADR 0119).

## What "on par" takes

At each join, the best of the four, and where TypeScript's best has a
loophole, not the loophole:

1. **A tag's element reaches its handlers and its `ref`**, as TypeScript's
   and Laminar's do.
2. **Event and tag maps in `webapi`**, as TypeScript's, from the same
   `@webref/events` and `@webref/elements` data, without a fallback that
   takes any string.
3. **Types survive the trip out**, at least as genType's do: enums with
   fields as `{TAG}` unions, closures with their parameters, `Map`, `Set`,
   `Result` and `Promise` as TypeScript's, opaque types branded.
4. **A typed value of unknown shape and a JSON type**, as ReScript's
   `unknown` and `JSON.t`, which also lets `webapi` keep `Response.json()`.
5. **Coverage**: more of WebIDL's interfaces, its other callback types, and
   its enums as string-literal enums.
