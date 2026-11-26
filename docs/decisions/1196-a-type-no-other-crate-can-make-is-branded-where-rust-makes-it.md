# 1196. A type no other crate can make is branded where Rust makes it

Status: Accepted (2026-11-24, #2403 step 2c; the lead's choices: brand at
construction, not at exits; the runtime brands what the declarations
brand). Extends [1187](1187-a-tuple-struct-no-caller-can-make-is-declared-opaque.md),
whose brand was the declarations' alone, and
[1194](1194-a-value-that-holds-others-is-checked-whole.md), whose checker
of such a type now tests it.

Case ([0262](0262-when-rust-and-js-disagree.md)): **C**, as 1192's.

## Context

A `pub` struct with a field that isn't `pub` is how Rust keeps an
invariant: only its crate's code makes an `Email`, so every `Email` passed
its check. ADR 1187 declares one opaque, so TypeScript takes one only of
what rust-js's JS gave. JS without TypeScript, or with `as unknown as
Email`, still gave Rust `["not an email"]`, which ADR 1194's checker took:
its fields were of their types. A field check can't hold an invariant:
chapter 15's `Temperature { celsius: -500 }` passes it.

## Decision

**What the declarations brand, `sealed` (ADR 1187, `declarations::contract`
its one answer), whose JS is an object of its own, is branded where Rust
makes one, and checked by its brand where JS gives one:**

```js
export function email(text) {
  return $brand("app#7e6d147c3a102184::Email", [text]);
}

function checkTemperature(value) {
  $checkBrand(value, undefined, undefined, "app#7e6d147c3a102184::Temperature", "a Temperature");
  $checkObject(value);
  $checkField(value, "celsius", $checkF64);
}
```

- **A brand is a `WeakSet` of the objects Rust made of the type** (O(1)
  to test, nothing JS can forge), one per type, not per instantiation:
  `Id<String>` and `Id<u32>` are one `Id` set, as the declarations' one
  `$idBrand`.
- **Found by the type's key in a registry every copy of the runtime
  shares**, `globalThis[Symbol.for(..)]` (ADR 1184, #2333): its crate's
  name, its disambiguator and its path, `app#7e6d…::Email`. A library
  bundled with its own runtime and the app beside it read one set, so a
  value one copy branded the other takes. Two versions of one crate have
  two disambiguators, two types to Rust, so two sets.
- **Its fields are still checked** after its brand, as JS can write into
  an object Rust made, `email[0] = 5`.
- **Branded where each maker makes one** (`boundary/brands.rs`'
  `branded`, called by each):
  - `adt_value`, every literal and constructor call, MIR's and THIR's;
  - a constructor as a value, `.map(Email)` (`constructor_value`);
  - a copy, `{ ...p }` (`copy_parts`), a `Copy` type's move and ADR
    1197's copies among them;
  - a clone (`clone_parts`);
  - a derived `Default`;
  - serde's reader (`serde/de.rs`' `construct`);
  - a `const`: its initializer, lowered as a body is, never rustc's value
    of it, which would be unbranded (`representation/consts.rs`).
  - A struct update, `{ ...base, x }`, is `adt_value`'s.
- **Not branded:**
  - **A `#[repr(transparent)]` newtype of a number or a string,** whose JS
    is its field: no `WeakSet` holds a primitive. It keeps its field
    check and the declarations' brand. (So react-in-rust's most made
    sealed type, `EventPriority` of a `Lane`, pays nothing.)
  - **One that always holds another** (`always_holds`), whose object is
    its field's: branding it would brand the field's value.
  - **std's,** `Ipv4Addr` or an iterator's adapter, whose declarations
    are std's own (ADR 1149), not `sealed`'s, which declares a crate's.
  - **A library's type, checked by its consumer**: a consumer's function
    of one checks it as any library type, not yet; the library's own
    entry checks it, by the one registry.

## Cost

- `WeakSet.add` is ~25–30 ns a construction, against 1.5–9 ns to make the
  object; the key's lookup adds ~7–10 ns; `has` is ~1 ns. Every Rust
  construction of a branded type pays it, as a brand only Rust's makers
  give is the point.
- React's port makes 14 sealed types at 30 sites; the hottest it brands
  is the hooks' `Effect`, one an effect a render.
- A module-level set, read once, would take the lookup off each
  construction: not yet, as its declaration's plumbing outweighs 8 ns.

## Why

- **Rust never reads a value of a type no other crate can make that its
  crate didn't make.** A JS caller can't forge an `Email`, nor a
  `Temperature` of -500.
- **At construction, not at exits:** a missed exit refuses a value Rust
  made, which is worse than a forgery missed. Each maker is listed above,
  and the guard test holds each.
- **One predicate:** the runtime brands exactly what the declarations
  brand, by `sealed`, so the two can't drift.

## What holds it

- `test/boundary-brands.test.ts`, the guard: JS's own refused; each maker
  above, given to JS each way a value leaves (a return, in a `Vec`, an
  `Option`, an `Rc`, a callback's argument, a callback's result) and given
  back; a generic's one set; a primitive newtype's field check; a library's
  type its consumer clones on another copy of the runtime; two versions of
  a crate, two keys; serde's reader.
- A mutation of each maker's brand, of the checker's, and of the shared
  registry, `brands-per-copy`, each caught.

## Amendment: a sealed type's constant is rustc's value, branded (#2403, 2026-11-24)

A constant of a type no other crate can make is rustc's value of it with its brand's mark on its literal,
`$brand("..::NaiveDate", { yof: 49152 })`, not its initializer lowered where it's used: chrono's
`NaiveDate::AFTER_MAX` used in another module read `date.rs`'s private `MAX_YEAR` there, and bitflags'
`Perms::READ` called `Perms` inside its own literal. A module's constant keeps the mark through
`aggregates::const_object`'s layout. The crate corpus's snapshots keep a brand's key without its crate's
hash, `itertools#<hash>::..`, which Cargo's build metadata moves with each compiler.

## Amendment: a value of no size is branded as its constant is (#2403, 2026-11-26)

A value of a sealed type of no size that a consumer's copy of a library's generic code makes, where its
optimized MIR wrote none (`_0 = Checked(())` gone, #1889), is its constant's: `zero_sized_js` gives it
through `const_js`, which brands it as above, `$brand("..::Checked", [undefined])`. `zero_sized_value`
had a branch of its own that made it of its fields' values and branded it again: no test, probe or
corpus case, nor itertools' crate, told the two apart, the branch's mutant giving the same JS, so it's
gone, one owner of a sealed constant's brand. `--retire`'s run couldn't judge it: its port control
refused, React's port at its pin regenerating otherwise with #2403 itself. The guard test gains the
case, a proof token `check<N: AsRef<str>>(name) -> Checked` given a name that drops, so the consumer
compiles its own `check`.

A brand on a body's literal is its mark or the call around it, `$brand(key, { .. })`, printed alike:
no test or probe told them apart either (`unmarked-brand`'s mutant): a guard's literal, which tidy
elides, is of a value that never leaves Rust and so is never branded (`unreached-value-branded`). The
mark stays, one form with a constant's, which `aggregates::const_object` reads; the mutation of an
object literal's mark is now its brand left off, `object-literal-unbranded`, which the guard test's
literal catches.
