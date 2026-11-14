# 1153. A binding trait's projection is the TypeScript type its associated type writes

Status: Accepted (2026-11-15, #1817's second step, the forms the lead
approved).

Case ([0262](0262-when-rust-and-js-disagree.md)): none. Only declarations
change; the JS is what it was.

## Context

The vercel UI kernel's `on` adds a listener and returns what removes it.
Upstream (vercel-labs/ui@220c1a0,
`packages/core/src/foundation/kernel/events.ts:4-13`) writes it:

```ts
function on<Type extends keyof HTMLElementEventMap>(
  target: HTMLElement | Document | Window,
  type: Type,
  listener: (event: HTMLElementEventMap[Type]) => void,
  options?: AddEventListenerOptions
)
```

The port's Rust is generic over webapi's catalog, `Target:
Listen<Name>`, with its listener a `&dyn Fn(&<Target as
Listen<Name>>::Event)` ([ADR 0223](0223-webapi-event-and-tag-maps.md)).
Its declaration was `on<Target, Name>(target: Target, name: Name,
listener: (value: any) => void, ..)`: any name, any listener. Codex's
consumer (rust-js-lang/vercel-ui-sdk-in-rust@da281b6,
`tests/types/kernel-events.ts`) wants a button's click a `PointerEvent`
with `clientX`, a document's keydown a `KeyboardEvent` with `key`, and a
name no target fires refused.

## Decision

**A binding trait's associated type says what its projection is to
TypeScript with `#[rust_js::types]`, written in the trait's `Self` and
parameters**, and the compiler declares `<T as Trait<..>>::Assoc` as
that type, of the projection's `Self` and arguments
(`declarations/projections.rs`):

```rust
pub trait Listen<E> {
    #[cfg_attr(rust_js, rust_js::types = "@rust-js/webapi#EventMapOf<Self>[E]")]
    type Event;
}
```

```ts
import type { EventMapOf } from "@rust-js/webapi";

export function on<Target, Name extends keyof EventMapOf<Target>>(target: Target, name: Name, listener: (value: EventMapOf<Target>[Name]) => void, options?: AddEventListenerOptions | null | undefined): () => void;
```

- **A bound constrains its key.** A bound `Target: Listen<Name>` says the
  projection is a type, so each of the item's type parameters its type
  indexes by is constrained as TypeScript's own indexing requires, `Name
  extends keyof EventMapOf<Target>`, whether or not the signature names
  the projection: `listened<Target, Name>(.., listener: &dyn Fn(&Event))
  where Target: Listen<Name>` constrains `Name` too.
- **One attribute.** On a type, `#[rust_js::types]` says what it is; on a
  trait, what a type it bounds is (erasure, `C: ReactNode`); on an
  associated type, what its projection is. Not on the trait for the key:
  that would give one attribute two meanings there. Anywhere else, a
  function, a method, a constant, it's refused, never left unread. An
  associated type without one is `any`, as before.
- **The reader reads `M[K]`**, a property's type by its key's, beside
  `T[]` ([ADR 0811](0811-console-create-task-may-be-absent.md)); the model
  and its printer have `indexed` and `keyof`.

**The type function is generated from the impls, one catalog, and
ships with `@rust-js/webapi`.** The generator writes `webapi/index.d.ts`
of the `Listen` impls it writes, beside `src/lib.rs`; the crate's Cargo
package includes it, so its npm package does, `types: "index.d.ts"`, and
this repository's workspace of the crate is that package by name,
`@rust-js/webapi`, as `@rust-js/std` is std's. A consumer's
declarations `import type` it; nothing is copied into a `.d.ts`.

- **Each target's map is TypeScript's where the same**, else
  TypeScript's with what differs taken out and given as Rust's,
  `Omit<HTMLElementEventMap, "abort" | "error" | ..> & { abort: Event;
  .. }`, else its own where TypeScript has none; each form declared once,
  named after its shallowest target. Of TypeScript 7.1's maps and 242
  targets, 34 forms of their own, 43,298 bytes, pinned at 44 KB.
- **Exactly first, then a subtype.** TypeScript's types are structural:
  an `SVGSVGElement` extends an `SVGGElement`, which has no member of its
  own, so a chain of `T extends X` sent it to another target's map. A
  target is found by its type exactly first, `Is<T, X>`, mutual
  assignability, each form's targets shallowest first; then a type that
  extends one, a custom element's class, deepest first; else `{}`, of
  which no name is a key.
- **Two targets TypeScript can't tell apart take the shallower's map.**
  `XMLHttpRequestUpload` adds nothing to `XMLHttpRequestEventTarget`, so
  no type function tells them apart; Rust's upload `progress` is a
  `ProgressEvent`, its parent's `Event` (no spec fires one at the
  abstract parent, [ADR 0223](0223-webapi-event-and-tag-maps.md)'s
  fallback). It's declared the parent's, `Event`: looser for an upload's
  caller, and sound, as a listener of an `Event` takes a `ProgressEvent`.
- **A type TypeScript's DOM lacks** (`BatteryManager`,
  `HTMLSelectedContentElement`, `SpeechRecognition`) is an empty
  `declare global` interface of `index.d.ts`'s, in the exact chain only.
  A consumer's own declaration of it, `rust_js::declares`
  ([ADR 1152](1152-rust-js-declares-what-typescripts-dom-lacks.md)),
  merges with it, and its map follows; undeclared, it's `{}`, matching
  no other type. When TypeScript's DOM gains one, its interface merges
  with the empty one harmlessly.
- **A newer TypeScript's map.** The override form keeps TypeScript's
  map, so a consumer on a newer TypeScript whose `HTMLElementEventMap`
  gained a name accepts it where the catalog of this TypeScript lacks
  it: TypeScript's own lib typing a call whose JS is
  `addEventListener(name)` for any name, accepted (the lead's decision).
- **`focus` and `blur`** stay `Event` of an element (#1823): their maps
  are generated, not TypeScript's.

**Known difference:** the listener's parameter is `value`, upstream's
`event`: a `dyn Fn` names none, and a declaration names a parameter of
its type's ([ADR 0546](0546-a-declaration-names-a-function-as-its-js-does.md));
logged for the user's decision.

## Why

- **The type is the catalog's.** Each `EventMapOf<X>` is checked by tsc
  against the `Listen` impls of `src/lib.rs`, every target, so the
  declarations say what rustc lets the Rust do: not TypeScript's maps,
  which differ for 163 of 242 targets ([ADR 0223](0223-webapi-event-and-tag-maps.md)).
- **One place says it.** The impls and the map come from one generator
  run; the attribute names the map, and the compiler reads nothing of
  webapi by name.

## Alternatives

- **Each consumer's `.d.ts` declares the map it uses:** a global `type`
  doesn't merge, so two crates' would clash, and a module's copy would
  repeat 44 KB in each.
- **A deepest-first chain alone:** wrong for `SVGSVGElement`, as above.
- **Full maps of their own:** 67 KB, and none follows TypeScript's lib.
- **An attribute on the trait for the key:** a second meaning of
  `#[rust_js::types]` on a trait, where it already says what a type it
  bounds is.

## Consequences

- `test/webapi-event-map.test.ts`: Codex's consumer type-checks under
  strict tsc, and a narrower listener and a name of no target are
  refused, of a bound alone too; every target's `EventMapOf` is its
  catalog's map by tsc, or of a parent tsc can't tell it from, of
  lib.dom's own `extends`, and a wrong one fails; a consumer's declared
  `BatteryManager` gets its map, and an unrelated type none; the size
  pinned. `webapi-coverage`'s generator check covers `index.d.ts` too
  (`generatedBeside`), and a definitions bump runs the new test.
- `test/declarations-written.test.ts`: `#[rust_js::types]` on a
  function, a constant or a trait's method refused.
- Mutations bound-constrains-no-key, projection-of-the-trait-alone,
  projection-declared-any, indexed-access-unread and types-on-anything.
- [ADR 0103](0103-runtime-package.md)'s table: the webapi workspace is
  `@rust-js/webapi`.
