# Web platform coverage: rust-js against TypeScript and ReScript

Research date: 2026-10-08. A measurement, following
[type-foundations.md](type-foundations.md), whose fifth "on par" item,
coverage, was named there and not measured. The react.dev port kept finding
web APIs the `webapi` crate lacks (`URL.createObjectURL`,
`HTMLScriptElement.supports`, `history`), one at a time. This counts all of
them at once.

| Project | Revision |
| --- | --- |
| TypeScript `lib.dom.d.ts` (`tsc/internal/bundled/libs/`) | `1f70213d4` (2026-09-04) |
| @rescript/webapi | `b46bc2c` (2026-09-19) |
| ReScript Stdlib (`packages/@rescript/runtime`) | the checkout of type-foundations.md |
| rust-js `webapi` | `a5ccbc43`, after static methods were added |

## Method

- **TypeScript's DOM** is read with TypeScript's own parser
  (`@rust-js/typescript`). A class is a `declare var X: { prototype: X;
  new(..) }`; its members are its interface's properties and methods, those
  of the mixins it extends (`AnimationFrameProvider`, `ParentNode`, ..),
  not those of its parent class, and its statics and constructor.
  `on*` handler properties are counted apart.
- **rust-js and ReScript** are counted by the members their MDN links name,
  `docs/Web/API/<Interface>/<member>`, which both write on every binding.
  ReScript's record fields without one are missed, so its count is a floor.

## Numbers

```
                              TypeScript    @rescript/webapi    rust-js webapi
 classes                          692           263                 194
 members of TS's classes        6,715         1,915  (28.5%)      1,147  (17.1%)
   with statics, constructors   8,050             -               1,195  (14.8%)
 in the 194 classes rust-js has 2,609             -               1,083  (41.5%)
 on* handler properties           164            some                 0
 globals (not on*)                106             -                  74
```

```
 JS standard library        TypeScript lib.es*    ReScript Stdlib    rust-js js crate
 modules                    every ES built-in     71                 Unknown, Dict, Json, Promise,
                                                                     RegExp, string, number, json,
                                                                     object, ArrayBuffer, Uint8Array
 Date, Intl                 yes                   yes                no
 typed arrays               all 11                all 11             Uint8Array
 Symbol, WeakMap, WeakRef   yes                   yes (no WeakRef)   no
```

Rust's own std covers part of the last table: `HashMap` and `BTreeMap` are
JS `Map`s, `Vec` is an `Array` with its methods, `f64` has `Math`'s. What
Rust has no type for, a calendar date, a locale's formats, a typed array
other than bytes, isn't there at all.

**rust-js is behind both.** ReScript has 1.7 times its members, TypeScript
5.9 times. The gap isn't one cause:

## Why members are missing

From `webapi/generate.ts`'s own skip counts, and the classes the measure
finds absent:

1. **The allow-list.** `INTERFACES` names 198 interfaces; the other 498 of
   TypeScript's classes aren't generated (`Range`, `Selection`,
   `ShadowRoot`, `URLSearchParams`, `MutationObserver`, `ResizeObserver`,
   `WebSocket`, `Notification`, `Screen`, `CSSStyleSheet`, `IDBFactory`,
   `Crypto`, `CustomElementRegistry`, every `SVGAnimated*`, ..). Each member
   typed by one of them is skipped too: `SVGAnimated*` 279,
   `Attr` 7, `ValidityState` 7, `ShadowRoot` 2, `Range` 1, ..
2. **Callback attributes.** `on*` properties (`EventHandlerNonNull` 450,
   `OnErrorEventHandlerNonNull` 4, `OnBeforeUnload..` 3) and callbacks other
   than an event listener's: `requestIdleCallback`, and the observers'
   once those classes are in.
3. **WebIDL types the generator doesn't take:** `float` 23, `sequence` in
   a result 20, a union in a result 12, `FrozenArray` 5, `long long` and
   `unsigned long long` 4, `record` 1, `async_sequence` 1,
   `ObservableArray` 1.
4. **Constants and static attributes**: `Node.ELEMENT_NODE` and the other
   36 of `Node`, `static` attributes. Static methods came in `a5ccbc43`.
5. **The JS standard library** beyond what the port needed: `Date`, `Intl`,
   the other typed arrays, `Symbol`, `WeakMap`, `WeakRef`, `DataView`.

## What delivering takes

In order, each measured by this method so the number only goes up:

1. **A coverage check in the repository**, a test that runs this measure
   against a pinned `lib.dom.d.ts` and fails when a member that was covered
   isn't: the count is a ratchet, not a claim.
2. **Generate every class TypeScript has**: the allow-list becomes the set
   of interfaces `lib.dom.d.ts` declares, read from the same specs (adding
   the ones it reads that `SPECS` lacks). Most of cause 1 goes.
3. **The missing WebIDL types**: `float` as `f32`, sequences and frozen
   arrays in results as `Vec`, unions in results as their enums (ADR 0215
   has them for arguments), `long long` as `f64` (WebIDL's is a JS number),
   `record` as `Dict`, constants as constants, static attributes.
4. **Callbacks**: each callback type as its closure, and `on*` properties as
   an `Option` of one, as TypeScript has them.
5. **The js crate**: `Date` and `Intl` first, then typed arrays, `Symbol`,
   `WeakMap`, `WeakRef`, `DataView`: bindings to what JS has, as ReScript's
   Stdlib is.

Past that, matching TypeScript is matching what it generates from: the same
specs, so the same classes.
