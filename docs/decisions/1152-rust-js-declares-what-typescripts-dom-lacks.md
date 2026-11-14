# 1152. A bound type TypeScript's DOM lacks is declared of its binding, `#[rust_js::declares]`

Status: Accepted (2026-11-14, #1790, the form the lead approved).

Case ([0262](0262-when-rust-and-js-disagree.md)): none. Only declarations
change; the JS is what it was.

## Context

The Battery Status API, `navigator.getBattery()`, wasn't bound: the
generator reads a spec's interfaces only where TypeScript's DOM declares
them, or whole where `SPECS` names it ([ADR 0024](0024-web-crate.md)), and
`@types/web` has no `BatteryManager`. Read whole, it's bound, but a crate
whose function gives one declares `Promise<BatteryManager>`, a name no
`.d.ts` declares: strict tsc refuses the consumer's import, `Cannot find
name 'BatteryManager'`. HTML's `HTMLSelectedContentElement`, which
TypeScript hasn't caught up with, was the same already.

Two forms could say what the type is. A text the generator writes, a
`declare global` of its own beside the crate, would be a second mapping
of WebIDL to TypeScript beside the compiler's of Rust to TypeScript, the
one every other declaration is made by, and its catalog a second one to
keep in step with the bindings.

## Decision

The binding says it, and the compiler writes it. The generator places
`#[cfg_attr(rust_js, rust_js::declares)]` on each type it binds whose
name `@types/web` doesn't declare (`typescriptNames`, the reader that
already decides which members are kept), and on no other. A crate's
declarations that name such a type declare it once, of the binding, by
the one mapping of Rust to TypeScript:

```ts
declare global {
    interface BatteryManager extends EventTarget {
        readonly charging: boolean;
        readonly level: number;
        onlevelchange: ((event: Event) => void) | null;
        // ..
    }
}
```

Each `get` binding is a property, `readonly` where no `set` binding writes
it; each method binding is a method; what the type derefs to is what it
extends. A global augmentation, as TypeScript's DOM declares its classes,
so the name the consumer writes, `BatteryManager`, is the one the crate's
declarations name, and a second crate declaring it too merges into the
same interface rather than clashing. TypeScript's factory can't make
`declare global`, so the printer composes it (`typescript/index.js`,
`kind: "global"`).

`battery-status` joins `SPECS`, read whole.

## Consequences

- `navigator.get_battery().await.level()` compiles and its `.d.ts`
  type-checks under strict tsc; a consumer's `battery.charging = true` is
  refused, `TS2540`.
- `Navigator`'s own `getBattery` isn't added to TypeScript's `Navigator`:
  only a type the crate names is declared, and a member of a type
  TypeScript has is its own. A consumer's TypeScript that calls
  `navigator.getBattery()` itself, not through the crate, still lacks it.
- One placement rule: `test/webapi-declares.test.ts` holds that the marker
  is on exactly the bound types `@types/web` lacks, today
  `BatteryManager` and `HTMLSelectedContentElement`. A TypeScript release
  that declares one takes the marker away at the next regeneration.
- Mutations: `scripts/mutations/lower/declarations/globals.ts`.

## Amendment (2026-11-14): what the pinned TypeScript's own lib lacks

The marker is placed of the `lib.dom.d.ts` the pinned TypeScript ships,
which a consumer's `tsc` reads by default, not of `@types/web`, which may
be ahead of it: `@types/web` 0.0.366 declares `HTMLSelectedContentElement`,
which TypeScript 7.1.0-beta's lib doesn't. Ten types are marked:
`BatteryManager`, `HTMLSelectedContentElement`, `IDBRecord`, `NodeRange`
(ADR 0024, amended), SpeechRecognition's four and WebTransport's two. A
consumer that adds `@types/web` too has two declarations of it, which
TypeScript merges as one interface where their members agree.
