# 1172. The js crate's builtins written from TypeScript's library

Status: Accepted (2026-11-18, the lead's decision; #1904). Amends
[0283](0283-js-crate-covers-typescripts-es-library.md): its first group,
`Intl.Locale`, #1831's week info among it, is written from TypeScript's
library.

## Context

The js crate's bindings of JS's standard library were each written by
hand, as specs in `builtins/intl.ts`, `errors.ts` and `typed_arrays.ts`:
every member, its types and its optionality a second copy of what
TypeScript's library declares, kept in step by hand. So `Intl.Locale` had
none of what ESNext added, `getWeekInfo()` among it, which the Vercel UI
SDK's port needs (#1831), and nothing said so but coverage. Since #1903 the
library is one, the pinned engine's own ESNext selection, and every member
the crate doesn't bind is a gap named with its issue.

## Decision

**What's written from the library is the library's.** `builtins/library.ts`
reads a root's declarations from the engine's graph, merged across the
files TypeScript merges: its interface's members, its own and those of
what it extends that it doesn't redeclare, its constructor, the aliases and
interfaces its members name. Each member's name, type, optionality and
inheritance are the library's; no table repeats them.

**One way to write each binding form.** `builtins/render.ts` holds the
forms the hand specs wrote by, moved out of `builtins/intl.ts`: an enum of
a string union, an options struct each field an `Option` not given, a
record of what's read, a method by its link name, a sealed input of a
union, a class and its constructor's module. The hand specs and the
generator call it; nothing is copied.

**The types, by the existing rules.** A string is `&'a str` given in options
and `String` read; a boolean `bool`; a number `f64`, the native number; an
array read a `Vec`; a string union, through its aliases, its enum; a class
read `&'static` it; an interface read its record, written once. An
optional property, or a result `| undefined`, is an `Option` read
`none_undefined`, its `None` JS's `undefined`: a part a locale wasn't
given. Of `String` and `bool` results `null` never arrives, so `none_undefined`
reads as `== null` would; it's the representation the result is, not a
check a test tells apart.

**What the library doesn't say, `builtins/policy.ts`.** Each entry the fact
TypeScript lacks, why, and the test that proves it, which the entry cites:
- names: `Intl.LocaleCollationCaseFirst` is `CaseFirst` and
  `Intl.LocaleHourCycleKey` is `HourCycle`, the established Rust names the
  collator's and the date formatter's options share, their members the
  aliases'; `Intl.TextInfo.direction`'s inline `"ltr" | "rtl"` is
  `TextDirection`; `new Intl.Locale(tag)`'s `string | Intl.Locale` is the
  sealed `IntoLocaleTag`, not `IntoLocalesArgument`, whose lists a tag isn't;
- what throws: `new Intl.Locale` throws ECMA-402's `RangeError` of a tag
  that isn't one, so it's a `Result`.
An entry never narrows a declared union, an optional key or an overload.

**A form it doesn't bind is refused** by the member's name and its file, a
`Refused`, never written as `any` or narrowed: a test gives it a made-up
class whose method gives `any`.

**A group is written from the library, or by hand, never both.**
`Intl.Locale`'s spec in `builtins/intl.ts` is deleted, its group written by
`writeClass(library, "Intl.Locale")` in its place, and `CaseFirst` and
`HourCycle` by `writeUnion` where the shared unions are. The other groups
keep their specs until each is moved, then each spec goes.

## What moved

`Intl.Locale`, as the user approved the boundary (#1904):
- `locale::new(tag: impl IntoLocaleTag, options: Option<&LocaleOptions<'_>>)`,
  a tag of text, owned or borrowed, or another locale; options `None`;
- `LocaleOptions.base_name`, the library's;
- each part it wasn't given read `None`, `numeric() -> Option<bool>`;
- `get_calendars`, `get_collations`, `get_hour_cycles`,
  `get_numbering_systems`, `get_text_info() -> TextInfo { direction }`,
  `get_time_zones() -> Option<Vec<String>>` and
  `get_week_info() -> WeekInfo { first_day, weekend }`: no `minimalDays`,
  which only a comment of the library's names, and no legacy `weekInfo`.
The three callers, two tests and coverage's Intl arguments, now say
`locale::new(tag, None)`; no alias of the old signature is kept. Coverage
is 756 of 1,083, the seven getters bound and their gaps gone.

## Proof

test/intl-locale.test.ts: Rust callers of each member, run on Bun, each tag
form and the `RangeError`'s name; a locale's options given and not, its
parts and `numeric`'s `false`; its week, `firstDay` and `weekend` alone,
its text's direction of `en-US` and `ar`, its time zones, `undefined` of a
locale with no region; a strict TypeScript caller of what rust-js declares
of them, `Intl.Locale` and `Intl.WeekInfo` themselves; the generator's
refusal. Node gives the same and also the legacy `weekInfo` getter, which
isn't bound. Mutations of builtins/library.ts and builtins/policy.ts, each
caught by what's committed being what the generator writes or by a row.

This is case N of ADR 0262: the bindings' authority changes, not what JS
does.

## Amendment (2026-11-18): each class is an object's binding, by one owner

The vercel-ui SDK's port reads an older host's `weekInfo` as upstream does,
`tag as Intl.Locale & { weekInfo?: .. }`, `unsafe { js::cast(Some(
js::unknown(tag))) }`, and rustc refused it: no `Intl` class this
generator wrote was `Defined`, which `js::unknown` takes, nor `WeakKey`, so
no `WeakMap` could hold a locale or a formatter. webapi's generator wrote
both of each interface by a loop of its own, the typed arrays' template
`Defined` alone, and `lib.rs` a hand list of `WeakKey`s; `Map`, `Set`, the
weak types and node's `Buffer` had no `WeakKey`. Nothing said what a class
binding is, so a new generator could miss it, as this one did.

A class's instances are objects, so a binding of one is what every
object's is: `scripts/object-markers.ts`'s `OBJECT_MARKERS`, `Defined` and
`WeakKey`. webapi's generator and the builtins' (`render.ts`'s `classed`,
the typed arrays' template, which now writes the buffers' and the views'
`WeakKey` the hand list did and `ArrayBuffer`'s `StructuredClone` beside
them) write them by `objectMarkers`; test/object-markers.test.ts holds each
crate's `Classed` type to them, those written by hand too, and the hand
ones missing were added. webapi's lines are now `js::Defined`, the one
form.

`Defined` changes what an `Option` of an `Intl` object is: the object
itself or `undefined`, no box, as every object's is (ADR 0051); an object
is never `undefined` nor `null`, so nothing reads otherwise (ADR 0262's
first case). Row: test/intl-locale.test.ts's "an Intl object is an object",
`js::unknown(locale)` cast and a `WeakMap<Locale, u32>`, run on Bun.
Mutations `object-markers-weak-key-forgotten` and `classed-not-an-object`.
