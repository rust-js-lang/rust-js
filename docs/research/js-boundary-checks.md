# Boundary checks: what compile-to-JS languages do with a value JS hands them

Research date: 2026-11-24, for #2403 (check every value JS gives Rust
against its Rust type). This is a source review of shallow clones at the
revisions below, plus a probe of ReScript's released compiler. Citations
are `repository/path:line` at those revisions. Bug reports are GitHub
issues, read with `gh`.

| Project | Revision | Commit date |
| --- | --- | --- |
| Scala.js | `9afbf239a119a9b24addadbb5ee153ee022ac769` | 2026-11-22 |
| scala-js-website (its docs) | `f1a6534ac0e8fa279c0a197249408c23470ed459` | 2026-11-18 |
| ReScript (compiler) | `45dba2c9137d6be765c1d8d45f287d739e028498` | 2026-11-23 |
| rescript-lang.org (docs) | `6e40dc7dfa4100dcd3b78d2e6a2aa4a6d09626b2` | 2026-11-04 |
| ReScript, probed | npm `rescript@12.3.1` | |
| wasm-bindgen | `e692fe24a361915d9abb3fe88e0f591ea1fa5088` | 2026-11-17 |
| Kotlin | `bd17a4715fead09c6812a335a29a5c9808393c00` | 2026-11-24 |
| kotlin-web-site (docs) | `718396953364c6d6eba536d9de48acf2d1feed05` | 2026-11-23 |
| js_of_ocaml | `7ccb6deab9b0b18762cb59f6380450e55972a88c` | 2026-11-23 |
| Fable docs (fable-compiler.github.io) | `301e0f3a49c8f7ef12694b2fac62a0f4b19feb04` | 2026-10-30 |
| PureScript documentation | `80d27da23390c356c67800cda8c4f8ded54447b6` | 2026-11-13 |
| Gleam language-tour | `234cb0284d91c458fa26e1b66e4b0276597d722f` | 2026-08-07 |
| Gleam website | `4ccd6e8ce5300d1b1ab89c896205abfab0d2fd02` | 2026-11-17 |
| elm/core | `65cea00afa0de03d7dda0487d964a305fc3d58e3` | 2021-04-12 |
| elm/json | `2865dfce97a75724a75583a214d3a287d2abecd4` | 2025-09-19 |
| Rust's `core` (rust-src of 1.100.0) | `0ef1bc386` | 2026-11-09 |

## Conclusion

No language that keeps a type's invariants lets JS break them for free:

- **The two closest to rust-js, Scala.js and wasm-bindgen, never let typed
  code see an impossible primitive.** Where they don't check, they coerce
  into the type's domain: Scala.js's production `x | 0` for an `Int`, and
  wasm-bindgen's ToInt32 wrap for a `u32`.
- **They check, in every build, what can't be coerced:** an enum's
  discriminant, a `char`, and a class instance in wasm-bindgen's case.
- **Their type checks (`typeof`) run only in development:** Scala.js's
  `fastLinkJS` and wasm-bindgen's `--debug`.
- **Elm checks everything in every build**, through decoders; it has no
  FFI at all.
- **ReScript, Kotlin/JS, js_of_ocaml, Fable, PureScript and Gleam check
  nothing.** Their docs say the types are trusted, and that untrusted data
  goes through a decoder or an `unknown` type first.

rust-js does neither today. It doesn't check, and it doesn't coerce: a
`u32` parameter given `-1` holds `-1`. That is an impossible value that
none of the checking languages lets in, and its results are wrong answers,
not just odd values (#2391).

The measured costs (max agent, below) are small: about 2 ns a scalar, under
1 ns an element. The value is shown by the bugs checks found elsewhere,
mostly in hand-written bindings. The recommendation is #2403's design
with three changes; see [Options for rust-js](#options-for-rust-js).

## The languages

### Scala.js: checked in development, coerced in production

**What and when.** A value JS gives Scala code is cast to the Scala type.
This covers an exported method's parameters, which go through
`fromAny`, and a facade's result:

- `scala-js/compiler/src/main/scala/org/scalajs/nscplugin/GenJSExports.scala:749`: `fromAny(jsArg, param.tpe)`, an exported method's parameter;
- `scala-js/compiler/src/main/scala/org/scalajs/nscplugin/GenJSCode.scala:5807`: `fromAny(boxedResult, …)`, a native facade member's result;
- `GenJSCode.scala:5285-5304`: `fromAny` is `makePrimitiveUnbox` for a primitive, and `genAsInstanceOf` for anything else.

How the cast lowers is the `asInstanceOfs` semantic, one of seven
configurable "undefined behaviors":

- `scala-js/linker-interface/shared/src/main/scala/org/scalajs/linker/interface/Semantics.scala:21-28`: the seven settings;
- `CheckedBehavior.scala:25-27`: the three modes, `Compliant`, `Fatal` and `Unchecked`.

| Mode | What it does | Source |
| --- | --- | --- |
| `Fatal` | Checks, and throws an `UndefinedBehaviorError` | `CoreJSLib.scala:612-617` |
| `Compliant` | Checks, and throws the JVM's `ClassCastException` | `CoreJSLib.scala:612-617` |
| `Unchecked` | Coerces | `SJSGen.scala:548-559` |

- **Checked.** `$uI(v)` accepts a number that is an `Int`, or `null`,
  which becomes `0`; any other value goes to the throw
  (`CoreJSLib.scala:1508-1523`).
- **Unchecked.** An `Int` is `expr | 0`, a `Double` is `+expr`, a `Boolean`
  is `!!expr`, a `String` is `expr || ""`, and a class type passes as it
  is (`SJSGen.scala:548-559`).

**Defaults.** The modes differ by build:

- `Semantics.Defaults` is `Fatal` for every check except `moduleInit`
  (`Semantics.scala:269-278`).
- The full stage's linker config applies `.optimized`
  (`sbt-plugin/src/main/scala/org/scalajs/sbtplugin/ScalaJSPluginInternal.scala:496-498`,
  `fullOptJS / scalaJSLinkerConfig`). `fullLinkJS` runs that stage's
  settings (`ScalaJSPluginInternal.scala:364`).
- `.optimized` turns every `Fatal` into `Unchecked`, and leaves `Compliant`
  as it is (`CheckedBehavior.scala:18-21`, `Semantics.scala:73-81`).
- So development (`fastLinkJS`) checks, and production (`fullLinkJS`)
  coerces.

A facade type, a trait extending `js.Any`, is never checked:
"`asInstanceOf[T]` is completely erased for any `T` inheriting from
`js.Any`, meaning that it does not perform any runtime check"
(`scala-js-website/doc/interoperability/facade-types.md:88-90`).

**Configurable.** Yes, per semantic, for example
`_.withAsInstanceOfs(CheckedBehavior.Compliant)`, which "will have
(potentially major) performance impacts"
(`scala-js-website/doc/semantics.md:140-147`).

**Rationale.** From `scala-js-website/doc/semantics.md`:

- Why the checks are undefined behavior: "checking them is typically too
  expensive" (lines 107-112).
- Why the modes are split: `Fatal` "so that bugs can be detected more
  easily during development", and "in production code (`fullLinkJS`), the
  checks are removed for maximum efficiency" (lines 123-128).
- Why the error can't be caught: `UndefinedBehaviorError`s are not matched
  by `NonFatal`. "It is *never* OK to catch an `UndefinedBehaviorError`…
  since that means your program will behave differently in `fullLinkJS`
  stage than in `fastLinkJS`" (lines 130-136).
- The benchmarks page says the same: "in the `fullOpt` configuration, we
  remove the checks for undefined behaviors"
  (`scala-js-website/doc/internals/performance.md:23,32`).

**Bugs where the boundary bit.** Each was found by the development check.
In production, each would have been silent:

- **scala-js-dom#137:** `TimeRanges.start` was declared `Int`, and the
  browser gave `67.57`: "UndefinedBehaviorError: … 67.57 is not an
  instance of java.lang.Integer". `fullLinkJS` would give `67`.
- **scala-js-dom#139:** `PerformanceTiming` declared `Int`, and its
  millisecond timestamps overflow it. `| 0` would wrap them.
- **scala-js-dom#755:** a facade's `Response.type` was `undefined`, and
  the check refused it as a `ClassCastException`.

These three are bindings: the facade said a narrower type than the browser
returns.

### wasm-bindgen: coerce what any bit pattern holds, check what would be UB

All of these are in `wasm-bindgen/crates/cli-support/src/js/`, the glue it
generates for each export.

**Numbers.** `Int32ToWasm` calls `assert_number` only
(`binding.rs:1876-1880`), and `assert_number`, `assert_bigint` and
`assert_bool` return at once unless `config.debug` is set
(`binding.rs:1407-1429`). In debug, `_assertNum` checks `typeof` only,
not range or wholeness (`mod.rs:3015-3022`).

So the value reaches Wasm through the JS API's ToInt32. The guide states
the rule (`guide/src/reference/types/numbers.md:20-22`): a number is
"rounded towards zero", NaN and Infinity become 0, and "if the rounded
number is too large or too small for the target integer type, it will wrap
around". A BigInt wraps too (`numbers.md:44-46`). A `u32` going out to JS
is `>>> 0` (`binding.rs:1881-1890`).

**Bool.** `typeof` in debug only (`binding.rs:2061-2066`).

**Checked in every build:**

- **A class instance:** `_assertClass`, `instanceof`, throws "expected
  instance of …" (`binding.rs:1371-1375`, `mod.rs:3998-4003`). A by-value
  receiver must also be the exact class (`binding.rs:1388-1405`).
- **Non-null:** `_assertNonNull` (`binding.rs:1507-1510`).
- **A `char`:** `_assertChar` refuses a surrogate or a value past
  `0x10FFFF` (`binding.rs:1512-1515`, `2068-2074`; `mod.rs:4327-4333`).
- **A numeric enum:** `try_from_abi` matches each discriminant, and
  anything else throws "invalid enum value passed"
  (`crates/macro-support/src/codegen.rs:5559-5576`).
- **A string enum:** a binding's string enum has a hidden `__Invalid`
  variant that an unknown string becomes, so Rust code has to handle it
  (`codegen.rs:2397-2401`, `2437-2443`).

**Strings.** They are copied through `TextEncoder`, which replaces an
unpaired surrogate with U+FFFD. "That means the string in Rust is now
different from the string in JavaScript!" The guide points to
`js_sys::JsString` to keep it exact (`guide/src/reference/types/str.md:24-45`).

**Configurable.** One flag, `--debug`: "Generates a bit more JS and Wasm
in 'debug mode' to help catch programmer errors, but this output isn't
intended to be shipped to production" (`guide/src/reference/cli.md:61-64`;
`crates/cli/src/wasm_bindgen.rs:55`, off by default).

**Rationale.** The split follows Rust's own rule. What would be undefined
behavior in Rust is checked always: an invalid discriminant, an invalid
`char`, a pointer to the wrong class. What only gives a surprising but
valid value is coerced, with a `typeof` in debug.

- **#2269 (2020):** `take_char_by_value('😃'.charAt(0))` made an invalid
  `char`, since `from_u32_unchecked` had no check. That's why `_assertChar`
  is unconditional now.
- **#1270 (2019):** an `&Element` called with no argument gave
  `TypeError: getObject(...) is undefined`. alexcrichton replied: "We
  don't provide a ton of guard rails in debug mode, but we do provide
  some… generate more type guard glue whenever `--debug` is passed."

**Bugs where the boundary bit.**

- **#1348 (2019):** the lossy surrogate replacement broke
  rust-dominator's `<input>`. An IME's two `input` events each carry half
  a pair, so each half became U+FFFD and the text was corrupted.
- **#2269:** the UB above.

### ReScript: nothing checked; a wrong value takes another branch

An `external`'s types are trusted. The docs say "if you're certain about
what an external function returns, simply assert the return value…
Otherwise use `unknown`". An `'a` return "means 'anything you want it to
be'… if the property is missing or contains something unexpected, your
code might break"
(`rescript-lang.org/apps/docs/markdown-pages/docs/manual/bind-to-js-function.mdx:352-364`).

`@unboxed` and untagged variants are told apart at run time by `typeof`
and `instanceof` (`variant.mdx:380`). Coercion between variants and
primitives "is always zero cost", with no check (`variant.mdx:604`).

**Probe.** Eight exported functions, compiled with `rescript@12.3.1` and
called from Node with values outside their types:

```rescript
type status = Open | Shipped | Lost
let label = (s: status) => switch s { | Open => "open" | Shipped => "shipped" | Lost => "lost" }
let label2 = (s: [#a | #b]) => switch s { | #a => 1 | #b => 2 }
let double = (n: int) => n * 2
let add = (n: int, m: int) => n + m
let len = (s: string) => String.length(s)
@unboxed type num = Int(int) | Str(string)
let show = (x: num) => switch x { | Int(i) => Int.toString(i) | Str(s) => s }
let first = (xs: array<int>) => xs[0]
```

| Call | Result | What happened |
| --- | --- | --- |
| `label2("c")` | `1` | Compiled to `s === "b" ? 2 : 1`, so an unknown tag takes `#a`'s arm. |
| `label("Gone")` | `undefined` | A `switch` with no `default:`; `undefined` is not a `string`. |
| `double(3.5)` | `6` | `n << 1` |
| `double(2 ** 31)` | `0` | |
| `add("5", 1)` | `51` | `"5" + 1 \| 0` |
| `len(5)` | `undefined` | Typed `int`. |
| `show(true)` | `true` | Typed `string`. |
| `first([])` | `undefined` | Typed `option<int>`, whose `None` is `undefined`; no wrong value here. |

Every row is a value of the wrong type or a wrong answer, and nothing
reports it.

**Configurable.** No.

### Kotlin/JS: the export is the function itself

An exported function is exported by name, `JsExport(name.makeRef(),
alias)`, with no wrapper, so nothing runs before its body
(`kotlin/compiler/ir/backend.js/src/org/jetbrains/kotlin/ir/backend/js/jsexport/ExportModelToJsStatements.kt:70-84`).

`Byte`, `Short`, `Int`, `Float` and `Double` are all a JS `Number`
(`kotlin-web-site/docs/topics/js/js-to-kotlin-interop.md:385-388`), so an
`Int` parameter given `3.5` holds `3.5` until an arithmetic `| 0`.

There are three escape routes, none checked:

- `dynamic` "basically turns off Kotlin's type checker"
  (`dynamic-type.md:14-18`);
- a cast to an external interface compiles with an "Unchecked cast"
  warning (`js-interop.md:207`);
- `unsafeCast<T>()` does "no type checking… at all"
  (`js-interop.md:213-215`).

`as` checks a class type, but that is a cast the code writes, not a
boundary.

**Configurable.** No.

### js_of_ocaml, Fable, PureScript, Gleam: trusted by the docs' word

- **js_of_ocaml:**
  - An OCaml `int` is a JS number, a `bool` 0 or 1, a constant variant a
    number (`js_of_ocaml/manual/runtime-representation.mld:14-25`).
  - A `string` is bytes, not UTF-16. JS strings are a type of their own,
    converted with `Js.string` and `Js.to_string` (lines 68-77).
  - `Js.export` hands JS an object's methods as they are
    (`rev-bindings.mld:6-27`); the caller "must convert" (lines 77-82).
  - `Js.Unsafe.global` and `js_expr` "are untyped"
    (`javascript-interop.mld:286`).
- **Fable:** a cast with `unbox` or `!!` "will bypass the F# type checker
  but please note **Fable will not add any runtime check to verify the
  cast is correct**"
  (`fable-compiler.github.io/docs/docs/javascript/features.md:1652`).
- **PureScript:**
  - "The compiler cannot check that values defined in the FFI have the
    correct runtime representation… it is your responsibility"
    (`documentation/language/FFI.md:26`).
  - "Data returned from Javascript functions cannot generally be trusted
    to be defined and non-null… it is important to sanitize data",
    through `Foreign` (`documentation/guides/FFI.md:196-198`).
- **Gleam:**
  - "Gleam trusts that the type given is correct so an inaccurate type
    annotation can result in unexpected behaviour and crashes at runtime.
    Be careful!" (`language-tour/src/content/chapter5_advanced_features/lesson06_externals/en.html:17-20`).
  - The compiler "cannot verify that the function" matches its annotation
    (`website/pages/documentation/externals.djot:82-85`).
  - A JS caller of Gleam's constructors "must be the correct type"
    (`externals.djot:1043`).

None of the four is configurable. Each sends untrusted data through a
decoder (`Foreign`, `gleam/dynamic`, `Js.Unsafe`'s conversions) the code
writes itself.

### Elm: everything decoded, in every build

Elm has no FFI. A value enters through flags or ports, and each is run
through a decoder:

- **Flags:** `_Json_run(flagDecoder, …)` at `init`, and a failure crashes
  (`elm/core/src/Elm/Kernel/Platform.js:37-38`).
- **Ports:** every incoming port value goes through its `converter`, and
  `__Result_isOk(result) || __Debug_crash(4, name, result.a)`
  (`Platform.js:457-462`).
- **Production too:** only the message differs. The debug build throws
  "Trying to send an unexpected type of value through port…"
  (`Debug.js:257-260`); production throws a hint URL (`Debug.js:233-236`).

Its `Int` decoder checks `typeof`, then a 32-bit whole number, then any
finite whole number (`elm/json/src/Elm/Kernel/Json.js:45-55`). `Bool` and
`String` are a `typeof` each (`Json.js:57-61`, `73-79`). The cost is a
walk of the value, as #2403's deep checks are.

**Configurable.** No. The decoders are the language's design.

### Rust itself: preconditions checked in debug builds

Rust's own answer for a broken `unsafe` precondition is the same split:
`assert_unsafe_precondition!` "is enabled at runtime if debug assertions
are enabled" (`core/src/ub_checks.rs:6-10`), and "in release builds"
the gate "will codegen to `false`" (`ub_checks.rs:56-60`).

A JS caller of an export is in the position of `unsafe` code: it promises
the types. The difference is what breaking the promise does:

- **Native Rust:** memory UB.
- **rust-js:** JS stays memory-safe, but the Rust program gives answers no
  Rust program could. AGENTS.md ranks that the worst bug.

## Summary

| | Checks | When | A bad value | Configurable |
| --- | --- | --- | --- | --- |
| Scala.js | `typeof` and instance tests at every cast: exports, facade results | Dev (`fastLinkJS`) | Dev: an uncatchable `UndefinedBehaviorError`. Prod: coerced (`\| 0`, `+x`, `!!x`) | Per semantic |
| wasm-bindgen | Enum, `char`, class, non-null | Always | Throws | No |
| wasm-bindgen | `typeof` of numbers, bigints, bools | `--debug` | Throws in debug. Otherwise wraps or truncates into the type | `--debug` |
| wasm-bindgen | Strings | Never | Lossy: U+FFFD | No (`JsString` to keep it exact) |
| Elm | Everything, by decoder | Always | Throws | No |
| ReScript, Kotlin/JS, js_of_ocaml, Fable, PureScript, Gleam | Nothing | | Impossible values, wrong branches | No |
| rust-js today | Nothing | | Impossible values (`u32` of `-1`) and wrong answers (#2391) | No |

## rust-js's own data (max agent, #2403's step 2)

Cost:

| Check | Cost |
| --- | --- |
| A `u32` entry check | 1.60 to 2.24 ns a call |
| `isWellFormed` | about 0.03 ns a character |
| A deep `Vec<u32>` | 12 ns at 10 items, 0.63 µs at 1,000, 62 µs at 100,000 (about 0.6 ns an element) |
| A struct of five fields | 18 ns |
| A brand | `WeakSet.add` 25 to 30 ns, plus 7 to 10 ns for the registry; `has` about 1 ns |

- **Output size:** the React port's output grew 2.02%, before brands.
- **What the checks found:** no bug in the port's own code. About 20
  refusals, all false: places where the compiler's contract and its
  representation disagreed, and test inputs outside the declared contract.
- **Pending:** the props-check cost per render on the port's React suite,
  checks on against off.

Those 20 refusals are evidence both ways. They found no user bug. They did
find compiler mismatches, the class Scala.js's `Fatal` mode found in
scala-js-dom. They also show the risk of a check refusing a valid program
when the contract is wrong.

## Rust's invariants that JS can break

These are what make rust-js's case worse than "odd values". JS stays
memory-safe, but Rust code assumes each of these, and its result without
them is a wrong answer:

- **Integer range and wholeness.** A `u32` of `-1` or `3.5` makes `n * 2`,
  a comparison, an index and an overflow check each answer differently from
  any `u32` (#2391's table: a `-5` quantity sums to `4294967293`).
- **Enum variants.** An unknown tag takes some `match` arm, or none. A
  lowering that turns the last arm into `else` takes it, as ReScript's
  `label2("c")` did.
- **Non-null and `Option`'s form.** `null` where a struct is expected is a
  `TypeError` deep in the body, not at the call. A `null` given to an
  `Option<T>` whose form tests `undefined` alone isn't its `None`.
- **UTF-8.** rust-js's `str` byte counts assume a well-formed string. A
  lone surrogate gives lengths and slice bounds no Rust `str` can have.
  wasm-bindgen's lossy replacement bit users (#1348).
- **A private-field struct's invariants.** JS can build an object with the
  fields and skip the constructor (#2403's open question on brands).

## Options for rust-js

Each option is costed with the numbers above.

1. **No checks; the guide says so (#2391).** This is ReScript's and
   Kotlin's model.
   - Cost: zero.
   - Risk: impossible values and wrong answers, with no error.
   - It goes against "a wrong answer is the worst bug", and the precedent
     is the languages that don't keep invariants. Not recommended.
2. **Coerce scalars into their type in every build; check what can't be
   coerced; check nothing deep.** This is wasm-bindgen's model:
   - coercion: `>>> 0`, `| 0`, the sign-extending shifts, `Math.fround`,
     `!!`;
   - checks: enum tags, `char`, non-null and `Option`'s form.
   - Cost: near zero for the coercions, about 2 ns for each check.
   - Rust never sees an impossible scalar. But the caller's `-1` silently
     becomes `4294967295`: still not what JS meant, and nothing says so.
   - It leaves deep values, struct fields and `Vec` elements, unchecked.
3. **Check scalars and shapes in every build, deep checks in development
   only.** This is Scala.js's model.
   - Prod cost: about 2 ns a value.
   - Dev catches the binding mismatches scala-js-dom hit, and prod keeps
     its speed for large inputs.
   - Downside: two behaviors. Scala.js mitigates that by making the error
     uncatchable, so code can't come to depend on it.
   - It needs a dev/prod split rust-js doesn't have for semantics today,
     a second way that "one way" asks to justify.
4. **Check everything in every build, as #2403 designed.** This is Elm's
   model.
   - Cost:
     - per call, about 2 ns a scalar and 18 ns a struct of five;
     - per item of a collection, about 0.6 ns;
     - in size, 2% of the port;
     - per render, the props cost (pending).
   - `#[rust_js::trusted]` skips it on a measured hot path.
   - Rust's meaning is kept for every input, which is ADR 0262's case C.

**Recommendation: option 4, with three changes from #2403.**

- **Check binding results as strictly as export parameters, and test
  those checks first.**
  - The evidence that checking pays is in bindings: scala-js-dom#137,
    #139 and #755 were facades declaring a narrower type than the browser
    returns.
  - rust-js's webapi and react crates are hand-written the same way.
  - Exports, by contrast, found nothing in the port.
- **Treat a false refusal as a compiler bug, like the ~20 max agent found.**
  Every refusal in the corpus, the port and the crate corpus is either a
  wrong contract to fix or a test input outside the contract to change,
  before the checks ship.
- **Keep it one behavior in every build; `#[trusted]` is the only
  opt-out.**
  - Measured costs don't call for Scala.js's production switch: 2 ns a
    scalar and 0.6 ns an element. Scala.js's switch was made when it called
    checking "typically too expensive"; its benchmarks page compares the two stages whole, optimizer and Closure included, not the checks alone (`doc/internals/performance.md:23-36`).
  - A dev/prod split would be a second semantics to keep consistent.
  - If the props measurement shows a render cost that matters, the
    fallback is option 3's split for deep checks only. Never option 1's
    unchecked scalars.

The error stays a foreign exception that `catch_unwind` doesn't catch,
as #2403 has it. That is Scala.js's reasoning for making
`UndefinedBehaviorError` uncatchable: a program mustn't come to rely on a
check's throw.
