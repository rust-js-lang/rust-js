# 0102. Two binding crates, `js` and `webapi`, named as ReScript's

Status: Accepted. Amended by [0215](0215-webapi-unions.md): a union is its
untagged enum, so a function has no other forms of one, and a constructor
no family of sources. Amends [0024](0024-web-crate.md), whose `web` crate is now
`webapi`, and [0035](0035-results-and-throwing-js.md). Amended by [0443](0443-a-value-a-caller-may-leave-out-is-an-option.md): an optional argument is an `Option` of the one function, where it was a `_with_` form of its own. Extended by [0860](0860-the-iterator-protocol-and-a-loop-of-what-its-test-makes.md): JS's iterator protocol, `js::Iterator<T>`.

Case: N ([0262](0262-when-rust-and-js-disagree.md)).

## Context

The pilot (ROADMAP M3.3) declared bindings the web crate should have given
it: `fetch` with a method, a body and a signal, `addEventListener` with a
signal, and `encodeURIComponent`. Every app would write the same ones.

Two things were missing, of two kinds:

```
                     the browser's APIs (WebIDL)        the JS language's globals
                     fetch, AbortController, ...        encodeURIComponent, ...
Scala.js             scala-js-dom                       its library: js.URIUtils
ReScript             @rescript/webapi                   its standard library: Stdlib_Global
rust-js              web ✓, but no options objects      nothing
```

- **The browser's options objects:** WebIDL's dictionaries, `RequestInit`.
  The generator (ADR 0024) made a struct of one only when a function
  returns it, and left out every argument of one.
- **The language's own globals:** `encodeURIComponent` isn't in WebIDL, so
  no generator makes it. The web crate held some of JS by hand already,
  `Promise`, `JsError`, `RegExp`, `spawn`.

ReScript keeps the two apart, and its webapi (`experimental-rescript-webapi`)
was generated once, from TypeScript's DOM types, and is curated by hand to
names people call: the common form gets the plain name, `fetch(string,
~init=?)` and `fetchWithRequest`, an options object is a record of optional
fields, and constructors are `make`, one per source `from*`.

## Decision

**Two crates, as ReScript's two libraries:**

- **`js`** (`rust-js-builtins`): what JS has that Rust's `std` doesn't. `JsObject`,
  `Promise`, `JsError`, `RegExp`, `ArrayBuffer`, `Uint8Array` and `spawn`,
  moved from the web crate, and the URI functions: `encode_uri_component`,
  `encode_uri`, and `decode_uri_component` and `decode_uri`, which throw a
  `URIError` on what isn't encoded text, so return a `Result` (ADR 0035).
  Hand-written; much smaller than ReScript's library, since rust-js maps
  `std`'s strings, numbers and collections itself.
- **`webapi`** (`rust-js-webapi`), the web crate renamed, as ReScript's is:
  the browser, generated from WebIDL, using `js`'s types.

React's crate re-exports both, `react::js` and `react::webapi`, and the
native builder gives a program each of the three by name. As Cargo
dependencies (ADR 0101), all three are rustc's to check.

**`js::settle(promise)`** makes any promise's `.await` a `Result`, as a
binding returning `Promise<Result<..>>` is (ADR 0035): `webapi`'s `fetch`
rejects on a network error or an abort, and its `.await` would throw.
`settle(window::fetch(window, url)).await` is `await $settle(fetch(url))`.

**The generator follows ReScript's rules:**

- **An options object a function takes is a struct of `Option` fields**,
  as ReScript's is a record of optional fields: `None` is `undefined`, which
  a browser reads as not given, and with no field required it has
  `Default`, so `RequestInit { method: Some("POST"), body: Some(&json),
  ..Default::default() }`. What it borrows lives for its `'a`. A field JS
  names otherwise has `#[rust_js::name = "referrerPolicy"]`; inherited
  members are its own; one of a type Rust can't take is left out; one of a
  union is its string, where it can be one; `FIELD_TYPES` gives the rest,
  a `RequestInit`'s `headers` a `Headers`.
- **A function's other forms say what sets them apart**, the most common
  keeping the plain name, as `PRIMARY` has it: `window::fetch(window,
  url)`, `fetch_with_request` and `fetch_with_init`. An optional argument
  adds a `_with_<argument>` form, Rust having none; a union's options object
  is named after its argument, the rest after their types:
  `add_event_listener_with_options` and `add_event_listener_with_bool`.
- **Constructors:** one signature is `new`, `event::new("ping")`, and its
  optional arguments `new_with_<argument>`; a family of sources is
  `from_<type>`, `web_assembly_module::from_array_buffer`, with ReScript's
  names where a type can't give one, `request::from_url`. Rust's `new` is
  ReScript's `make`.

The curation is data in the generator (`PRIMARY`, `RENAMES`,
`FIELD_TYPES`), so the crate is still regenerated when WebIDL changes;
ReScript's is edited by hand after generating.

## Why

- **The language and the browser are different things:** a program for
  Node uses `js` and not `webapi`, as a ReScript one uses its standard
  library and not webapi. And no WebIDL generator can make what isn't
  WebIDL.
- **Names are for the call people write.** WebIDL's first overload isn't
  the common one: `fetch(Request)` came first, and a URL is what most calls
  pass.

## Alternatives

- **The globals in `webapi`:** one crate, but the browser's and the
  language's in one, which ReScript and Scala.js each keep apart.
- **`stdlib`, ReScript's name:** read as Rust's `std`, which rust-js maps
  too.
- **Hand-editing the generated crate, as ReScript does:** each regeneration
  would have to redo the edits.
- **`make` for constructors, ReScript's:** Rust's is `new`.

## Consequences

- **Every use was renamed:** `web::` is `webapi::`, and what moved is
  `js::`; `window::fetch_with_str` is `window::fetch`, `request::new_with_str`
  `request::from_url`. The playground loads `libjs.rmeta` beside the others,
  from `/crates/`.
- **The pilot declares no bindings of its own** but Sonner's: `fetch` with
  an init, a listener removed by its signal, and `encode_uri_component`
  are the crates'.
- **Tests:** `test/async.rs` posts with a `RequestInit` and removes a
  listener by its signal; `test/throws.rs` round-trips the URI functions,
  a malformed one an `Err`, and settles a rejected promise.
- **An options object's fields not given are written out as `undefined`**,
  `{ method: "POST", body, headers: undefined, .. }`. Leaving them out is a
  change of rust-js's objects: its `$eq` compares their keys.

## Amendment: the `js` crate is `rust-js-builtins`, in `builtins/`

It was `rust-js-js`. crates.io has no scopes, as npm has, so a crate's
package name is its whole name there, and once published, it's kept for
good: it's named for what it holds, JS's standard built-in objects, as MDN
calls them, apart from the browser's, `rust-js-webapi`'s, and from Rust's
`std`. So is its directory, `builtins/`, as `react/` and `webapi/` are
theirs. Its library is still `js`: a program says `js::Promise`, beside
`webapi::Element` and `react::use_state`.

## Amendment: JSON, `replace`, `Object` and an `Error`'s message

The playground declared both for itself, and every program writing JS text
would: `json::stringify(text)` quotes a string as JSON, which is a JS string
literal too, and `reg_exp::replace(text, pattern, with)` is
`text.replace(pattern, with)`. Each is exact for what it's typed for.
`JSON.stringify` of other Rust values isn't their JSON, as rust-js has them in
JS: `None` is `undefined` and an `i64` a `BigInt`, which it throws on;
that's serde's. A replacer closure and `matchAll` stay a program's bindings:
a JS replacer is given the match, then each group, then where it matched, so
its Rust type is its pattern's.

The rest of what the playground declared of JS's own is here too:
`object::from_entries`, a JS object of keys and values, `object::is`,
whether two JS objects are one, which `std::ptr::eq` isn't in rust-js, and
`js_error::is_error` and `js_error::message`, an `Error`'s, and
`number::to_fixed`, JS's `toFixed`, which rounds a tie away from zero, `2.5`
to `"3"`, where `format!("{:.0}", x)` rounds it to even, as Rust does. Not
`JSON.parse`:
what it gives is whatever the text holds, so no Rust type is its, and
serde's reader checks the text against one. What else it declared is a
library's (CodeMirror, the WASI shim, Sucrase), the browser's (webapi has
`addEventListener`'s options, not yet `performance`), or Rust's: a
`HashMap` is a JS `Map`. `test/builtins.rs`
checks each, and the playground uses them instead of its own.

## Amendment: timers are the builtins crate's

`set_timeout`, `clear_timeout`, `set_interval` and `clear_interval` take a
closure, where webapi's `set_timeout` of `Window` takes only a string of code
to run. They aren't the language's, but every JS runtime has them as globals,
browsers', workers' and Node's, and ReScript's standard library, which this
crate follows, has them: a crate that isn't only a browser's can use them.
What they give is opaque, `TimeoutId` and `IntervalId`: a number in a
browser, an object in Node. The playground uses `set_timeout` instead of its
own, and `object::is` instead of its own `Object.is` for a message's window.
What else it declares is a library's or its own: webapi has an iframe's
`contentWindow`, `MessageEvent` and `performance` now (ADR 0024), and
`object::is` takes two types, since one JS object may be seen as either,
a message's sender, an object, and a frame's `Window`.

## Amendment: a timer is cleared of an id or none

`clear_timeout` and `clear_interval` take an `Option`, as TypeScript
types `clearTimeout(id: number | undefined)`: what a JS program clears is
often a timeout it may not have set, react.dev's Preview's `timeout` its
listener sets. Two react.dev ports bound their own `clearTimeout` of an
`Option` for it, which a port shouldn't need to. ReScript's takes an id
only; TypeScript's type is what JS does.

## Amendment: a timeout takes `TimerHandler`, and a window's timers are gone

[ADR 1020](1020-a-timer-is-the-js-crates-global.md) (the user's decision,
rd10): `set_timeout`'s handler is TypeScript's `TimerHandler`, `string |
Function`, a function given as it is or text of code, and webapi's
`window.set_timeout`, `clear_timeout`, `set_interval` and `clear_interval`,
a second way of text alone, are gone, each an error naming the js crate's.


## Amendment: a dictionary given has what TypeScript requires (2026-11-01)

A dictionary a function gives, `TextEncoder.encodeInto`'s
`TextEncoderEncodeIntoResult`, had each member WebIDL doesn't mark
`required` as an `Option`, `read: Option<f64>`. But WebIDL's
`dictionary` members are optional for what a caller gives, and what the
browser gives has them all, as TypeScript's DOM types them: `read:
number; written: number;`. React's server stream config for Node reads
`const {read, written} = textEncoder.encodeInto(..)`, which the port wrote
`encoded.read.unwrap_or(0.0)`, a `?? 0` upstream lacks.

**A dictionary a function gives has each member TypeScript's DOM requires
as its own type** (`generate.ts`'s `dictionaryType`, of
`coverage.ts`'s `typescriptInterfaces`, which now says what each
interface requires): four of them, `TextEncoderEncodeIntoResult`,
`CryptoKeyPair`, `URLPatternResult` and `URLPatternComponentResult`. What
a function takes is as it was, its members `Option`s but WebIDL's
`required`.

Case ([0262](0262-when-rust-and-js-disagree.md)): A: the browser gives
each of them, so the `None` was never read.

Held by `test/webapi-objects.test.ts`'s "a dictionary a function gives has
the members TypeScript's DOM requires" and `test/web_forms.rs`'s
`into.written`.

## Amended (2026-11-01): `json::stringify` of any value

`json::stringify` took a `&str` alone, so a port stringifying anything
else wrote `stringify_with(value, None, 0)`, `JSON.stringify(value, null,
0)`, where React writes `JSON.stringify(value)`: ReactFizzHooks'
`JSON.stringify(keyPath)`, ReactFizzConfigDOM's import map, and
ReactPerformanceTrackProperties' arrays. **`json::stringify(value)` is
`JSON.stringify(value)` of any value `stringify_with` takes: a `String` of
a value whose JSON is exact, a `JsonText`, and of any value, an `Unknown`,
what may throw or be `undefined`, as `stringify_with` gives each. Case
([0262](0262-when-rust-and-js-disagree.md)): N, JS's own call. One way:
`stringify_with` is given a replacer or an indentation, never neither**,
which `test/architecture.test.ts` ("JSON of no replacer and no indentation
is stringify's") rejects, naming `json::stringify`, in the repository's
Rust, its tests' and the guide's. Held by `test/js-builtins.test.ts`
("unwrap of a throwing call each branch makes is each call") and
`test/webapi-bindings.test.ts`.

## Amended (2026-11-01): a `ReadableStream`'s source is TypeScript's (rd452)

WebIDL's `ReadableStream` constructor takes `optional object
underlyingSource`. The streams spec reads it as its `UnderlyingSource`,
whose functions are given the `ReadableStreamController` union, and
webapi bound it as a `&dyn Any`, so a Rust program couldn't say what it
gave. React's Fizz entries write `new ReadableStream({ type: 'bytes',
pull: (controller) => startFlowing(request, controller), cancel },
{highWaterMark: 0})`. **The constructor takes `Option<&dyn
IntoUnderlyingSource>`, of TypeScript's DOM's two sources: an
`UnderlyingByteSource`, `type: "bytes"`, whose `start` and `pull` are
given a `ReadableByteStreamController`, and an `UnderlyingDefaultSource`,
whose are given a `ReadableStreamDefaultController`. Each function is
lent, as a dictionary's callback is.** `webapi/generate.ts` writes them by
hand (`SOURCES_BY_HAND`), as no WebIDL declares them, and `PARAM_TYPES`
says which parameter takes them. The alternative, WebIDL's
`UnderlyingSource`, would give each `pull` the union to test, which React
doesn't write (the user's decision). Case
([0262](0262-when-rust-and-js-disagree.md)): N. Held by
`test/webapi-objects.test.ts` ("a byte stream reads from its byte
source's pull"). Amended by [1056](1056-a-streams-general-source-is-typescripts.md):
TypeScript's general `UnderlyingSource` too, of either kind, its functions
given a `ReadableStreamController`, either controller (rd403).

## Amended (2026-11-01): a record is what vouches it's one (rd453)

WebIDL's `record<K, V>`, TypeScript's `Record<string, V>`, wasn't bound.
A union with a record member left that member out: `HeadersInit` was its
sequence alone, `headers::new(Option<&[&[String]]>)`, and a parameter or
field of a record was skipped. React's Fizz writes `new
Headers(headersDescriptor)` of its `{ Link }`. **A record is `&dyn Record<Value
= V>`, of webapi's unsafe `Record` trait: a `js::Dict<V>` is one, and a
struct of `V`s or of `Option`s of them vouches it is, `unsafe impl Record for
HeadersDescriptor { type Value = String; }`. A union's record member takes
each of them, `IntoHeadersInit` of a `T: Record<Value = String>`.** It's
unsafe because what implements it says what JS reads of it. Each `None` is
written `None` where the struct is made, so it's left out (ADR 0280). One a
variable holds is `undefined`, and a record reads that as the text
`"undefined"`. The trait is webapi's own, so its blanket impls stand beside
each union's other members: no other crate may implement it for a type
that's one. Records elsewhere are bound now too: `ClipboardItem`'s items and
WebGPU's `constants`. Case ([0262](0262-when-rust-and-js-disagree.md)): N.
Held by `test/webapi-objects.test.ts` ("a struct that vouches it's a
record is one, new Headers(descriptor)").
