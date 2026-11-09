# 0229. A union parameter takes each member as it is

Status: Accepted. Amends [0214](0214-untagged-enums.md) and
[0215](0215-webapi-unions.md): a binding's union parameter is a sealed
trait of its members, where it was the untagged enum. Builds on
[0039](0039-generic-bindings.md) and [0100](0100-separate-crates.md).
Amended (2026-11-02, rd540): a union of text takes a `&String` and a
`String` too.

Case: N, D ([0262](0262-when-rust-and-js-disagree.md)).

## Context

A union parameter is its untagged enum (ADR 0215), so every call converts:

```rust
element::before(el, "text".into());     // el.before("text")
window::fetch(window, url.into());      // window.fetch(url)
```

TypeScript's `before("text")` and `fetch(url)` need nothing, and
`.into()` at every call of a union is the cost 0215 accepted. ADR 0214
said why a trait couldn't stand in: an `extern` function can't be generic.

A proposal was a wrapper of ordinary Rust over the binding:

```rust
unsafe extern "Rust" {
    #[link_name = "upload"]
    safe fn upload_raw(body: UploadBody<'_>);
}

pub fn upload<'a>(body: impl Into<UploadBody<'a>>) {
    upload_raw(body.into());
}
```

It doesn't fit how rust-js compiles crates:

- **A binding crate has no JS** (ADR 0024): `webapi` and `react` are
  declarations only. A wrapper's body would make `webapi` a module.
- **A caller never reads a library's bodies** (ADR 0100): each crate is
  compiled once, a generic function given dictionaries, and rustc gives no
  THIR for another crate. `upload("hello")` would be `upload("hello",
  dictionary)`, calling `webapi`'s `upload`, which calls `into` and then
  the browser's. Taking that back out is inlining across crates, which
  0100 rules out.

```
the wrapper:  app.js ──upload("hello", Into)──► webapi.js ──Into.into(x)──► upload(x)
the goal:     app.js ──upload("hello")──► upload
```

But ADR 0039 already made a binding generic: an ordinary function with
`#[rust_js::link_name]`, whose body is never compiled. The react crate's
attributes take a member trait so today, `draggable(value: impl
value::Booleanish)` (ADR 0228), as `children` takes an `impl ReactNode`.
A union's member is its JS value already (ADR 0214), so a binding that
takes one passes it on as it is: there's nothing to convert.

## Decision

**A union is still its untagged enum, for what's returned, a field, and
`match`; a binding's parameter of one is `impl` a sealed trait of its
members, which passes the value on as it is:**

```rust
/// What a `string | Blob` takes: each member, and the enum.
#[diagnostic::on_unimplemented(message = "`{Self}` is not a `string | Blob`")]
#[cfg_attr(rust_js, rust_js::types = "string | Blob")]
pub trait IntoUploadBody: Sealed {}
impl IntoUploadBody for &str {}
impl IntoUploadBody for &Blob {}
impl IntoUploadBody for UploadBody<'_> {}

#[cfg_attr(rust_js, rust_js::link_name = "upload")]
pub fn upload(body: impl IntoUploadBody) {
    unreachable!()
}
```

| Rust | JS |
|---|---|
| `upload("hello")` | `upload("hello")` |
| `upload(blob)` | `upload(blob)` |
| `upload(body)`, an `UploadBody` | `upload(body)` |
| `upload(42)` | error: `` `i32` is not a `string \| Blob` `` |

- **The trait is the enum's, `Into` and its name**: `IntoBodyInit` of
  `BodyInit`, `IntoNodeOrStr` of `NodeOrStr`. It's implemented for each
  type that converts into the enum, a member and an interface that
  extends one (ADR 0215), and the enum itself.
- **Sealed**, so no other crate adds a member: a member is the value
  itself only as 0214's tests tell it apart, which a type of a user's
  wouldn't be.
- **Not `Into<UploadBody>`**: a binding passes its argument on, so each
  conversion must be the value itself. 0214 checks that of a `From` into
  an untagged enum, but a user's own `impl Into<UploadBody> for T` would
  be skipped, and say nothing. A trait also says which members it takes.
- **`Option` of the trait only where `undefined` is "none"**, as a
  React attribute's (ADR 0228): `impl<T: IntoX> IntoX for Option<T>`.
  Not webapi's, whose `element::before(el, undefined)` would write the
  text "undefined"; an optional argument there is a form of its own (ADR
  0024).
- **The enum of a member, to `match` it**: `UploadBody::of(this: impl
  IntoUploadBody) -> UploadBody`, a binding of `"this"`, whose value is
  its parameter named `this`, so a user's own function that takes `impl
  IntoUploadBody` can tell what it was given.
- **To TypeScript, the trait is the union** (`rust_js::types`), as
  `ReactNode` is: a user's exported `fn send(body: impl IntoUploadBody)`
  is `send(body: string | Blob)`.
- **webapi's generator writes each trait beside its enum**, of the types
  that convert into it, with `of`, and a function or a setter with a union
  parameter as a binding of ADR 0039, an ordinary function, as an `extern`
  item can't be generic, as it does one of `any` (ADR 0225). Its other
  functions stay as they are. One `sealed::Sealed` seals all 22.
- **A field and a result stay the enum**: a field needs one type, so it's
  `RequestInit { body: Some(blob.into()), .. }`, as now.
- **A trait of another crate's passes no dictionary** (ADR 0100), so a
  user's `fn send(body: impl IntoBodyInit)` is `send(body)`, and nor does
  one of the crate's own, as it has nothing in it (ADR 0230): `fn
  kind(body: impl IntoUploadBody)` is `kind(body)`. (Amended: one of the
  crate's own passed one, `kind(body, IntoUploadBody)`.)

## Why

- **The JS is the call as written**, `upload("hello")`, with no wrapper,
  no dictionary and no conversion: a binding's arguments are passed on,
  and a member is its value.
- **It's the react crate's pattern already**, `impl ReactNode`, `impl
  StyleValue`, `impl value::Text`, made the rule for every union.
- **No new kind of type**: a `Union<A, B>` can't say `From` of both `A`
  and `B` (they may be one type), and `Union<String, &str>` has two Rust
  members of one JS kind, which 0214 checks a named enum for when it's
  declared.
- **It's tested**: `upload("hello")`, `upload(blob)`, `upload(body)` and
  an `Option`'s `upload(maybe)` are each the call as written in the JS,
  `upload(1.5)` is the error above; of the generated webapi,
  `element::before(el, "text")`, `response::new_with_body(blob)` and
  `window::fetch(window, url)` are `el.before("text")`, `new
  Response(blob)` and `window.fetch(url)`, a user's `kind(body: impl
  IntoBodyInit)` matches `BodyInit::of(body)` and is `kind(body)`, and its
  `.d.ts` is `kind(body: ReadableStream | Blob | … | string)`.

## Alternatives

- **A wrapper over the binding**, above: a module and a dictionary per
  call, unless inlined across crates.
- **`impl Into<UploadBody>`**: no conversion runs, so a user's `Into`
  would be skipped; and its error is rustc's, `` `From<i32>` is not
  implemented for `UploadBody` ``.
- **A `Union<A, B>` of the crate's own**: positional variants, overlapping
  `From`s, and members a JS value can't tell apart.

## Consequences

- `element::before(el, "text")`, `window::fetch(window, url)`: what 0215
  had cost every call is gone for parameters. A breaking change: a call
  that wrote `.into()` is now ambiguous, and drops it.
- webapi has a trait per union, and its functions of one are generic.
- A field of a union, and a result, still convert and `match` as now.

## Amended by

- [0373](0373-one-binding-form.md): A binding of a JS function is an ordinary function, and a JS type a struct

## Amendment: a union's `Function` member may be any function of a kind

[ADR 1020](1020-a-timer-is-the-js-crates-global.md): `js::set_timeout`'s
handler, TypeScript's `TimerHandler`, is `H: IntoTimerHandler`, whose
`Function` member is any function called once, `impl<F: FnOnce() +
'static> IntoTimerHandler for F`, beside `&str` and `&String`. A type
parameter bound by such a union is called as that kind
(`recognition::function_kind`).

## Amendment: a union of text takes a `&String` and a `String` (2026-11-02, rd540)

ADR 1061's amendment made a form's `append` take `impl IntoStrOrBlob`,
where it took `&str`. React's Flight reply client calls it of two
`String`s, `data.append(&stream_prefix, &part_json)`, 13 times: a `&str`
parameter took `&part_json` by its deref, but a generic parameter
doesn't deref, and `IntoStrOrBlob` had no `&String`, so each was rustc's
E0277, "`&String` is not a `string | Blob`".

**Each union webapi's generator makes of text takes a `&String` and a
`String` too**, as members of its trait beside `&str` (45 unions), as the
js crate's `IntoTimerHandler` already did: the value is the JS string
either way. A `String` is what an optional one's `Some(&text)` gives of
a `String` variable, `Option<&dyn IntoX>`. Case N: what JS is given is
the text.

Held by `test/webapi-objects.test.ts` ("a form's append takes text or a
Blob, an entry as it is"), of a `&String` and a `String`, run in Node,
and by React's port: its react-client crate's `cargo check`.

## Amendment: a function that gives back its argument gives its type (2026-11-10, #1103)

`crypto.getRandomValues(array)` fills the array it's given and returns
it. WebIDL types both as `ArrayBufferView`, a union, and webapi's
generator skipped it: a union it gives is an enum only where Rust takes
each member (ADR 0215), and `ArrayBufferView` has `Float16Array`, which
the js crate doesn't bind. Read as the enum, it would also have lost
what the caller passed, a `&Uint8Array`, to a `match`.

TypeScript's DOM says what WebIDL doesn't: `getRandomValues<T extends
Exclude<BufferSource, ArrayBuffer>>(array: T): T`. **A method TypeScript
declares as giving back a parameter, its result and that parameter's
type both one type parameter with an `extends`, gives the type the
parameter takes** (`givesBack`, webapi/coverage.ts, read by
webapi/generate.ts). A union's parameter is then a type parameter of
its trait, not an `impl`, so the result can name it:
`get_random_values<A: IntoArrayBufferView>(&self, array: A) -> A`. Any
other parameter's type is given as a function gives it, so `appendChild`,
`insertBefore`, `removeChild`, `replaceChild`, `adoptNode` and
`importNode`, the six others TypeScript declares so, are unchanged at
`&'static Node`. A type parameter with no `extends` says nothing:
`structuredClone<T = any>(value: T): T` gives a clone, and a clone of a
class's object is a plain object, so it stays `Option<&'static Unknown>`.

This is case N of ADR 0262: what JS gives back is the object it was
given, and Rust holds it as that object's type. The binding takes any
view, as WebIDL and TypeScript do. A view not of integers, such as a
`DataView` or a `Float32Array`, throws `TypeMismatchError` as JS's call
does. Only `getRandomValues` was skipped for this reason. The two other
operations skipped for a union result, `SubtleCrypto.generateKey` and
`exportKey`, give `CryptoKey | CryptoKeyPair` and `ArrayBuffer |
JsonWebKey`, whose dictionaries JS can't tell apart from an object, a
different rule.

Held by `test/webapi-crypto.test.ts` ("getRandomValues fills the array
it's given and gives it back"), run against the real API in Node and in
Chromium. The array given comes back as the same object, filled, and a
`DataView` throws. Also held by `test/webapi-coverage.test.ts`, whose
regeneration catches dropping the rule and dropping the `extends` test,
each tried by hand, as the mutations harness mutates the compiler alone.
