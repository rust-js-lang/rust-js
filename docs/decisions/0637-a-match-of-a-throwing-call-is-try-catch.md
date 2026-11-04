# 0637. A `match` of a throwing call's `Result` is `try` and `catch`

Status: Accepted. Amends [0035](0035-results-and-throwing-js.md), whose
`match` of a binding's `Result` was `$try` and its `TAG`, but for an `Ok`
arm that only gives the value. Amended by
[0639](0639-concat-text-of-any-value-may-throw.md): whether the call
threw, returned, `is_err()` or a `match` of `true` and `false`, is `try {
f(); return false; } catch { return true; }`. Amended by
[0660](0660-catch-unwind-catches-what-js-throws.md): `catch_unwind`'s
`Result` is `try` and `catch` by the same rule, and a throwing call's
`unwrap_or(d)` returned, `try { return f(); } catch { return d; }`.

Case ([0262](0262-when-rust-and-js-disagree.md)): N, nothing the program
can observe: the call is caught where Rust catches it and nothing else
is, the `Err` arm runs where its throw was, and the `Ok` arm where its
value was given.

## Context

A binding that may throw gives `Result<T, &JsError>`, its call `$try(() =>
f(x))`, `{ TAG, _0 }` (ADR 0035). React's `getComponentNameFromType`
catches a lazy component's `_init`:

```js
try {
  return getComponentNameFromType(init(payload));
} catch (x) {
  return null;
}
```

Its port writes the `match` a Rust developer writes:

```rust
return match init.call(payload) {
    Ok(resolved) => get_component_name_from_type(resolved),
    Err(_) => None,
};
```

and rust-js wrote `const call = $try(() => init(payload)); if (call.TAG
!== "Ok") { return null; } const resolved = call._0; return ..`.
Preparation wrote `try { return f(); } catch { .. }` only of an `Ok` arm
that gave the value as it was, `Ok(v) => v`, and of an `Err` arm that
didn't read the error.

## Decision

**A `match` of a throwing call's `Result`, its `if` of the `Ok` or the
`Err` either way and its `else` or what follows an arm that leaves, whose
arms read the `Result` by their bindings alone, is `try { .. } catch { ..
}` of the call: the `Err`'s binding is the `catch`'s, `catch (error)`.**
Only the call is in the `try`, as only it is caught in Rust:

- **What the `Ok` arm does is in the `try` where it can't throw**: what
  only makes values (`Stmt::only_makes_values`, the one answer of a
  statement's, which a panic's drops and a scope's `finally` asked of
  copies of their own), and a `return`, a `break` or a `continue` of
  what can't throw (`Expr::may_throw`). Its value is the call's, `try {
  return decodeURIComponent(text); }`, or its binding's, `try { const v
  = f(x); return $displayF64(v); }`.
- **Else it's after the `try`, where the `Err` arm leaves**, its value a
  `let` the `try` sets:

  ```js
  let resolved;
  try {
    resolved = init(payload);
  } catch {
    return null;
  }
  return getComponentNameFromType(resolved);
  ```

  React's own `try` holds `getComponentNameFromType`'s call too, which
  catches a throw of it as `init`'s; Rust's doesn't, so neither does
  this.
- **Else it's `$try`**, as before: an `Err` arm that goes on, with an
  `Ok` arm that may throw (`let doubled = match .. { Ok(v) => f(v),
  Err(_) => 0.0 }`), and a `Result` read otherwise, `?`'s `return
  result;`.

`if let Ok(v) = f() { .. }` and `let Ok(v) = f() else { return ..; }`
are the same `if` and are the same `try`. Preparation (`try_catch`)
writes it, the one place a `$try` becomes JS's `try`: lowering makes
every throwing call `$try` (`calls::catching`), and the `try` is a
rewrite of what's around it, as an ignored result's `try { f(); } catch
{}` is.

## Why

- **It's what a person writes**: React's `try`, `catch (x)` of what it
  reads, and no `{ TAG, _0 }` of a value the program never keeps.
- **Rust's semantics, exactly**: the `catch` is the `Err` arm, which runs
  of a throw of the call alone. A `let` the `try` sets is one statement
  more than React's, and the price of not catching what Rust lets
  through.

## Alternatives

- **The `Ok` arm in the `try`, as React writes it**: a throw of
  `getComponentNameFromType`, a panic among them, would be the `Err` arm's
  `None`. Rejected: Rust's `match` doesn't catch it.
- **A flag the `try` sets, `let ok = true; try { .. } catch { ok =
  false; .. }`, of an `Err` arm that goes on**: more than `$try`'s one
  line, which says it plainly.
- **Making the `try` where `$try` is made**, in lowering: the call
  doesn't know the `match` around it; MIR's arms are tidy's to shape.

## Consequences

- The guide's `decoded`, `try { return decodeURIComponent(text); } catch
  (error) { .. }`, and its `configLength`, the `let` and `try`; the
  `throws` snapshot's `sumJson`, `let numbers; try { numbers =
  JSON.parse(json); } catch (e) { .. }` and its loop after, and `uri`'s
  `try { decodeURIComponent(..); return ..; } catch (e) { .. }`; and the
  `todomvc` snapshot's `let Ok(storage) = window.local_storage() else {
  return ..; }`, `let storage; try { storage = window.localStorage; }
  catch { return []; }`. No corpus program changes. In
  `docs/readability.txt` the JS is a line longer, 88,080 to 88,081, the
  `let` of `todomvc`'s; no shape's share grows.
- `only_makes_values` answers what `drops.rs`'s `may_throw` and
  `unwinds.rs`'s `made_of_values` each answered of a statement: a
  statement of a value that can't throw, `x;`, now goes on a scope's run
  as it doesn't throw.

## What holds it

- `test/lowering-bindings.test.ts`, "a match of a throwing call's Result
  is try and catch": React's shape, a tail `match`, an `Err` arm first, a
  binding of the error, a value kept, and `$try` of an `Err` arm that
  goes on and of `?`, each run where the call throws and where the `Ok`
  arm's own code throws, which isn't caught.
- Mutations in `scripts/mutations/prepare.ts`.

## Amendment: an `Ok` arm that leaves, with nothing after it

**An `if let Ok(v) = f() { return v; }` with nothing after it in its
block is `try { return f(); } catch {}`**: its `Err` arm does nothing, and
what follows the block runs where the call threw, as it does after the
`try`. React's `describeElementType` writes it so of a lazy component,
`try { return describeElementType(init(payload)); } catch (x) {}`, which
its port's `if let Ok(name) = catch_unwind(..) { return name; }` wasn't:
the `Result` made, and its `TAG` tested, as only an `Ok` arm that goes
on, or one followed by statements that leave, were a `try`. Case N: the
same runs either way.

- `test/lowering-unwinds.test.ts`, "an if let Ok of catch_unwind that
  returns is a try whose catch does nothing", run where it throws and
  where it doesn't; mutation `ok-arm-leaving-alone-kept`.

## Amendment (2026-10-25): an `Ok` arm that leaves, what follows reading the `Err`

**`match catch_unwind(f) { Ok(()) => return, Err(e) => { .. } }` with
code after it is `try { ..f; return; } catch (e) { .. }` and that code
after the `try`**: what follows the `Ok` arm's `if` that reads the
`Err`, or what that declares, is the `Err` arm's, the `catch`'s, as ADR
0830's `let Err(x)` takes it (`caught_run`), and the rest runs after the
`try`, where the call threw, as it ran after the `match`. React's
`recursivelyFlushAsyncActWork` writes it so, `try { flushActQueue(queue);
queueMacrotask(..); return; } catch (error) { thrownErrors.push(error);
}`, which its port's `match` was `const value = $catchUnwind(..); if
(value.TAG === "Ok") { return; } const error = value._0;`. Case A: the
same runs either way.

Held by `test/corpus/catch_unwind_ok_returns.rs`'s `flushed`; mutation
`ok-arm-leaving-err-after-kept` (`prepare/catches.ts`).

## Amendment (2026-10-28): an `Err` arm that ends the function returns

Case ([0262](0262-when-rust-and-js-disagree.md)): A, the same runs
either way.

React's throwAndUnwindWorkLoop, `match catch_unwind(|| throw_exception(..))
{ Ok(did_fatal) => { if did_fatal { .. } } Err(error) => { if let
Some(return_fiber) = .. { ..; resume_unwind(error) } else { .. } } }` at
the end of its function (gap wl6 of the port), stayed a `$catchUnwind`
result tested `.TAG === "Ok"`: its `Ok` arm can throw, so it goes after the
`try`, which needs the `Err` arm to leave, and it ended without a `return`,
the function's end being its own.

**An `Err` arm that ends the function, nothing after the `match`, leaves
it**: the `catch` ends with `return;`, in each branch of a last `if` that
goes on (`ending_in_return`), and the `Ok` arm follows the `try`: `let
didFatal; try { didFatal = throwException(x); } catch (error) { if
(returnFiber != null) { ..; throw error; } else { panicOnRootError(x); return; } }`.
Preparation tells a function's body from a block (`block_ending`); a
`match` that ends an `if`'s branch goes on, so stays as it was.

Readability (ADR 0565): `return;` grows from 9.1 to 9.2 in 10,000 lines,
each a `catch`'s that ends a function, as upstream writes `panicOnRootError(..);
return;`, where a `$catchUnwind` or `$try` result and its `.TAG` test went.

Held by `test/lowering-unwind-arms.test.ts`, "an Err arm that ends the
function returns from its catch", run where each arm runs and where the
`Ok` arm throws; mutations `err-ending-function-goes-on`,
`err-ending-return-after-its-if` (`prepare/catches.ts`) and
`err-ending-a-block-returns` (`prepare.ts`).

## Amendment (2026-10-31): its test's `else` that goes on is the program's own

The nightly at 83234ef63 found `err-ending-function-goes-on` and
`err-ending-return-after-its-if` surviving. Their test's `unwind` writes
`panic_on_root_error(x); return;` in the `Err` arm's `else`, and since
ADR 0742's amendment of 2026-10-30 that written `return` stays where it is
written, so neither the rule nor its `else` case was reached there.

The test now also compiles `last`, a `match` that ends the function and
whose `Err` arm's `else` goes on, `panic_on_root_error(x)` with no
`return`, and holds its `catch` as `} else { panicOnRootError(x); return;
}` with no `.TAG`, run where each arm runs. Both mutations are caught
again. Case ([0262](0262-when-rust-and-js-disagree.md)): A, as above.

## Amendment: the `return` made of an `Ok` arm's is where that arm's is written (2026-11-02, rd525)

Case ([0262](0262-when-rust-and-js-disagree.md)): N: a comment's place,
which nothing runs, though ESLint reads it.

React's Flight client's parseModelString writes `if
(!mightHaveStaticConstructor.test(code)) { try { //
eslint-disable-next-line no-eval\n return (0, eval)(code); } catch (x) {
.. } }`. Its port writes `if !MIGHT_HAVE_STATIC_CONSTRUCTOR.with(..) &&
let Ok(function) = js::eval(&code) { // eslint-disable-next-line
no-eval\n return function; }`, which is `try { return (0, eval)(code); }
catch {}`. That `return` was placed at the call, in the `if`'s test, so the
comment above the written `return function;` had no statement to go
before. It printed above the `switch`, or nowhere, and ESLint's next line
wasn't the eval's. **The `return` made of an `Ok` arm's one statement,
written after the call's `$try`, is placed where that statement is**
(`try_catch`'s `held`), so the comment is right above `return (0,
eval)(code);`. The call keeps its own place where the arm is written
before it or nowhere, as `catch_unwind(|| f(x))`'s does.

Held by `lowering-comments-jumps.test.ts`'s "a comment above an Ok arm's
return is above the call's return in the try", run, and the mutation
`tried-return-at-the-call` (`prepare/catches.ts`).

## Amendment: an assignment made of an `Ok` arm's value is where the value is (2026-11-02, rd545)

Case N ([0262](0262-when-rust-and-js-disagree.md)): comments.

React's Webpack bundler config's addChunkDebugInfo, ported as `let href =
match catch_unwind(|| { // $FlowFixMe[incompatible-type]\n
url(..).href() }) { Ok(h) => h, Err(_) => .. }`, printed the comment above
`let href;`, where upstream has it inside `try {`, above `href = ..`. The
rule above placed what's kept of the `Ok` arm where that arm is written,
`h`, after the `match`'s call, so no statement started between the
comment and the arm, and it went above the whole `let`. **Only a `return`
the program wrote in the `Ok` arm is where it's written**; the assignment
of the arm's value, which the `let` makes, is where the value is, the
closure's call, after the comment. Held by
`lowering-comments-jumps.test.ts`'s "a comment in a catch_unwind closure
is above its value in the try", run, and the mutation
`tried-assignment-where-its-arm-is` (`catches.ts`). Since ADR 0776's
amendment (#462) a `catch_unwind`'s closure is called and applied arrows
place its body, so that test holds without this rule; a JS call's `$try`
still needs it: `lowering-unwind-arms.test.ts`'s "a try's assignment of a
JS call's Ok value maps to the call", `value = (0, eval)(code);` mapped to
the call's line, not the `Ok` arm's, is the mutation's test now.

## Amendment: a comment above the `Ok` arm is above its value in the `try` (2026-11-02, rd630)

Case N ([0262](0262-when-rust-and-js-disagree.md)): a comment's place,
which nothing runs, though ESLint reads it.

`let value = match js::eval(code) { // eslint-disable-next-line
no-eval\n Ok(v) => v, Err(_) => None }` printed the comment in the
`catch`, above `value = undefined;`, and `catch_unwind(|| parse(s))`'s
the same. The `Ok` arm's value `v` has no JS of its own, so the comment's
next start was the `Err` arm's value, and nothing between said otherwise.
Where the value's statement, `value = (0, eval)(code);`, is mapped stays
the call's, the `match`'s line (the amendment above), and the `try`
starts there too, so a comment keyed to its start would be printed above
the `try`, the first node there.

**The statement made of the `Ok` arm's value is of the `match`'s head
through that arm** (`try_catch`'s `held`, its span's end the arm's;
`js::applied_tried`'s, of a closure's value, the same): its Rust really
is that, not the `Err` arm's. The comment placement then needs no fact
of `try`s, only three general rules of where a comment's Rust is
(`to_oxc.rs`'s `comments_of`, ADR 0444):

- a comment isn't attached past a whole match arm, `Ok(h) => h,`, to the
  next arm's start (`openings::whole_arm`): it's the arm's, and an arm
  with no start of its own is inside the statement its value is of;
- one inside a statement, before nothing printed of it, is before the
  innermost statement it's in, as before;
- where that statement starts where one it's in starts, its comments are
  at a place of its own, the statement's (`placed::own_places`), mapped
  to nothing, as the outer one's start maps the same place.

So it's `try { // eslint-disable-next-line no-eval\n value = (0,
eval)(code); } catch { value = undefined; }`, and a comment above the
`Err` arm stays in the `catch`. The map test of the amendment above
holds. A statement given a key past its arm, a `Comments` block in the
`try`, was tried first and dropped: it's printed after the value by
where each starts, and it kept `js::applied_tried` from inlining the
closure, `href = (() => parse(s))();`.

Held by `lowering-comments-jumps.test.ts`'s "a comment above a match's Ok
arm is above its value in the try", run, both paths and an `Err` arm's
comment, and the mutations `held-assignment-without-its-arm`
(`prepare/catches.ts`), `tried-closure-value-without-its-arm`
(`js/applied.ts`), `comment-attached-past-a-whole-arm`,
`inner-comment-at-the-outer-start` (`to_oxc.ts`), `whole-arm-unseen`
(`to_oxc/openings.ts`) and `own-place-not-given` (`to_oxc/placed.ts`).

## Amendment: a comment above an `if` of the `Err` is first in the `catch` (2026-11-03, #489)

Case ([0262](0262-when-rust-and-js-disagree.md)): N, comments only.
`let r = catch_unwind(..);\n // What threw is logged.\n if r.is_err() {
log.push(9); .. }` printed `try { .. } catch { log.push(9); .. }` with
the comment nowhere: the `try` is where the call's `let` is, above the
comment, and the `if` it was above, whose comments they were, is gone.
**A comment right above an `if` of the `Err` the program wrote, `if
r.is_err()` or `if let Err(e) = r`, is the `catch`'s, first in it**, as
the arm it tests is, before the payload's binding too, `const e =
$unwound(thrown);`, which is placed where its `if let` binds it: a
block-of-comments marker (`StmtKind::Comments`, ADR 0753) at the `if`,
first in the handler, as ADR 0660's rd634 amendment gives a flag's
`catch`.

Which `if` the program wrote is lowering's to say, not the JS's shape:
MIR's first test of a `match`, `match r { // c\n Ok(()) => {} Err(_) =>
log.push(3) }`, is an `if` of the `Err` too, placed at the first arm's
pattern, and the comment there is that arm's, the `Ok`'s, never the
`catch`'s. Tidy's last stage marks each `if`'s test whose condition the
program wrote, as THIR's `if`s tell (`Written::ifs`), `Known::Written`,
by the rule `written_ors` already matched an `if` to its condition by,
now one function, `logicals::written_if`; preparation's `try_catch` asks
only that. Deciding it from where the comment or the test is written was
rejected: an `if let`'s test and a `match` arm's are both at a pattern.
A fact on the test, as `Known::Else` and `Known::Guard` are, is how a
later pass is told what the program wrote of an `if`. `Facts` is a `u64`
now: `Known` has 33 facts.

Not of an `if` of the `Ok`: `// c\n if let Ok(n) = r { return n; } 0` is
`try { ..; return 5; } catch { return 0; }`, the `Ok` arm folded into
the closure's `return`, and a marker first in the `try` is printed at
its end, by where each statement starts, after `return 5;`. That comment
is still lost, as is one above an `Ok(()) => {}` arm of a `match` the
`try` folds, and one above a `let Ok(n) = r else { .. }`; each is left
for its own change (#489's remainder).

Held by `test/lowering-comments-inside.test.ts`'s "a comment above a
folded is_err test is first in its catch", run: `is_err()`, `if let
Err(e)` read as thrown, a `match` whose `Ok` arm has the comment, of
something and of nothing, and an `if let Ok`'s, not put in the `catch`;
and the mutations `err-test-comment-unsaid`, `arm-comment-caught`,
`ok-test-comment-caught`, `thrown-binding-before-comment`
(`prepare/catches.ts`) and `written-tests-unsaid`
(`lower/mir/tidy/logicals.ts`). `arms-returned-comment-lost`, which a
marker of every `Err` test made redundant, is caught again.

## Amendment: a comment above an `Ok` test is before what its arm does (2026-11-03, #489)

Case ([0262](0262-when-rust-and-js-disagree.md)): N, comments only.
The last amendment left three comments lost, each above an `Ok` test the
`try` folds: `// c\n if let Ok(n) = r { return n; } 0`, `let Ok(n) = r
else { return 0; }`, and a `match`'s `// Fine.\n Ok(()) => {}`. **A
comment right above an `Ok` test is before what its arm does, where that
runs**: `try { ..; // c\n return 5; }`, the closure's value its
`return`; `try { ..; const n = 5; // c\n const twice = ..; }`, in the
`try`; `} catch { return 0; } // c\n log.push(n);`, after it, where the
arm can throw. A `let Ok(n) = r else`'s, above its binding, which is
written before its test, is before where the `try` sets it, `// c\n n =
5;`. An arm of nothing's is at the `try`'s end, `log.push(1); // Fine.\n
}`, where it ran.

Each is a block-of-comments marker (`StmtKind::Comments`, ADR 0753) that
`try_catch` places: one at the `if`, before the arm's own statements, or
with none, before the statement the call's value is set by; and, of a
binding written before its test, one at it, before that statement. The
`Err` side is the last amendment's, first in the `catch`. Of an arm
whose value the `try` sets through it, `// c\n Ok(v) => v,` of `let
value = match ..`, rd630's place of its own holds its comment, so none is
placed. A loop's `try`
whose closure's `return;`s are its `Ok` arm's jump (wl4,
`prepare/labels.rs`) keeps the marker before that jump.

Whose arm a comment right above the `if` is, is one question for both
sides: of an `if` the program wrote (`Known::Written`), the arm it runs;
of MIR's of a `match`, the arm its test names, `r.TAG !== "Ok"` the
`Ok`'s, `r.TAG === "Err"` the `Err`'s. MIR tests a `match`'s first arm
first, at its pattern, where the comment above that arm is, and names
that arm's variant, whichever way it tests; `js::result::tested`, the
`Result`'s owner, reads which. So a comment above a `match`'s first arm
of the `Err` is the `catch`'s, as one above an `if` of it is; reading
the arm from the test's way, as the last amendment rejected, would put
`// Fine.\n Ok(()) => {}`'s in the `catch`.

The printer had to print the markers where they're placed. oxc prints a
marker's comments, which no node starts at, by place: before the first
statement printed that starts past them, or at their block's end. A
closure's value starts before the comment, `5` of `|| { ..; 5 }`, and a
`catch`'s `return 0;`, which starts past it, is printed before what
follows the `try`. So **comments a block holds before a statement are
that statement's**, which oxc prints them before however it starts
(`starts::statement_spans`, the map `comments_of` attaches them by); only
those a block ends with are printed where they are.

Held by `test/lowering-comments-inside.test.ts`'s "a comment above a
folded Ok test is before its arm", run: an `if let Ok` returned, done in
the `try` and done after it, a `let Ok(n) = r else`, and a `match`'s
`Ok(()) => {}`; by the corpus's `tried_loop_jumps`,
`catch_unwind_thrown_named` and `catch_unwind_value_in_place`, unchanged;
and the mutations `ok-arm-comment-unsaid`, `ok-value-comment-unsaid`,
`ok-arm-comment-first-in-try`, `ok-arm-comment-after-value`,
`ok-arm-comment-after-try-unsaid`, `arm-set-comment-said-again`
(`prepare/catches.ts`),
`held-comments-by-place` (`to_oxc/starts.ts`) and
`said-blocks-loop-jump` (`prepare/labels.ts`). The last
amendment's `arm-comment-caught` and `ok-test-comment-caught` now break
the rule above. Its `written-tests-unsaid` is held by `// c\n if
!started.is_ok() { .. }`, an `if` the program wrote of the `Err` whose
test names the `Ok`, `started.TAG !== "Ok"`: of MIR's, that'd be the
`Ok`'s arm.

## Amendment: a first arm of nothing holds its own comment (#509)

A comment above a first `Ok(()) => {}` is now held by that arm's own marker
(ADR 0753's #508 amendment), which a loop's `match catch_unwind(..)`, whose
`if` MIR tests of the `Err`, needed: the comment is at the `try`'s end, not
in the `catch`. The mutation `arm-comment-caught` is gone with it, as no
program tells it from the code now (see there).

## Amendment: a comment above an `if` whose condition makes the call is the call's (2026-11-04, rd736)

Case ([0262](0262-when-rust-and-js-disagree.md)): N, comments only.
React's checkEvalAvailabilityOnceDev, `// eslint-disable-next-line
no-eval\n if js::eval("null").is_err() { console::error(..); }`, printed
`try { (0, eval)('null'); } catch { // eslint-disable-next-line no-eval\n
console.error(..) }`: the #489 amendment took a comment right above an
`if` of the `Err` for the `catch`'s. That rule is of a `Result` made
before the comment, `let r = f(); // c\n if r.is_err()`, the comment
between the call and its test. Where the `if`'s condition makes the call,
the call is written past the comment, as a `let` of it is: **a comment
right above an `if` whose condition makes the call, `if f().is_err()`,
`if let Err(e) = f()`, `if f().is_ok()` and `let Ok(v) = f() else` too,
is the call's, first in the `try`**, before it, neither arm's:
upstream's `try { // eslint-disable-next-line no-eval\n (0, eval)('null');
} catch { .. }`, where ESLint's next line is the eval's.

Whether the call is in the condition is where it's written against where
the `if` is: at or past it. The call's own place, not its `const`'s, which
of a temporary MIR made is an arm's: a `match catch_unwind(..) { // c\n
Ok(()) => {} .. }`'s test is at that arm's pattern, past the call, and its
comment stays the arm's. The marker is the `try`'s first statement, and
the `try` is where the `if` is, the call's statement where the call is: a
comment is printed before the first node at the place it's of, which
would be the `try` were both at the call.

Held by `test/lowering-comments-jumps.test.ts`'s "a comment above an if
whose condition makes the call is before the call in the try", run:
`is_err()`, `is_ok()`, `if let Err(e)`, a `let Ok(v) = .. else` (its
comment once), and `let r = ..; // c\n if r.is_err()`, still the
`catch`'s; and the mutations `condition-call-comment-caught`,
`condition-call-comment-unsaid`, `condition-call-try-at-the-call`,
`condition-call-where-its-const-is` and `tail-try-said-kept-a-closure`
(`prepare/catches.ts`), the last of a `try` that ends a function, whose
closure is its body past the marker (corpus `catch_unwind_ok_returns`).

## Amendment: a comment above an `if` of a closure's `catch_unwind` is above the `try` (2026-11-04, #688)

Case ([0262](0262-when-rust-and-js-disagree.md)): N, comments only.
The amendment above made a comment right above an `if` whose condition
makes the call the call's, first in the `try`. That is right for a call
of no body of its own, `js::eval("null")`, whose comment ESLint reads as
the eval's. But where the call is a closure the program writes,
`catch_unwind(|| { .. })` or `catch_unwind(AssertUnwindSafe(|| { .. }))`,
the closure's body is the `try`'s, and the Rust has two places for a
comment: above the `if`, and inside the closure. Upstream puts the first
above the `try` at six sites in React, ReactDOMSelection.js's getOffsets
(`// .. See\n // https://bugzilla.mozilla.org/show_bug.cgi?id=208427\n
try { .. } catch (e) { return null; }`, lines 26-33), ReactFiberHooks.js's
runActionStateAction and ReactAct.js's act among them; they printed first
in the `try`. Now **a comment right above an `if` whose condition is a
closure's `catch_unwind`, `if catch_unwind(|| ..).is_err()`, `is_ok()`,
`if let Err(e) = ..` and `let Ok(v) = .. else` too, is above the `try`**,
neither the call's nor an arm's, and one inside the closure is first in
the `try`, as it was. A call of no closure body keeps the amendment
above's placement, `try { // eslint-disable-next-line no-eval\n (0,
eval)("null"); }`; so does `catch_unwind(step)` of a function, whose call
is `step();`, as the program wrote no body there.

Whether the call is a closure's is `js::called_body` of it, the closure
of nothing called where it's made that `catch_unwind` lowers to (ADR
0660). The `try` is where the `if` is, as of the amendment above, with no
marker in it: a comment is printed before the first node at the place
it's of, the `try`. The marker before a `try`'s body that `tail_tries`
stepped past (rd736) is now only of a call of no closure body, which it
never takes apart, so that step is gone, and its mutation
`tail-try-said-kept-a-closure` with it.

Held by `test/lowering-comments-jumps.test.ts`'s "a comment above an if
whose condition is a closure's catch_unwind is above the try", run:
`is_err()`, `if let Err(e)` with a comment inside the closure too,
`is_ok()`, `let Ok(v) = .. else` (its comment once), and
`catch_unwind(step)`, still before the call in the `try`; and the
mutations `closure-condition-comment-in-the-try`,
`condition-call-taken-for-a-closure` and
`closure-let-else-said-in-the-try` (`prepare/catches.ts`).

The mutation `condition-call-where-its-const-is`, which reads where the
call is written from its `const` alone, `made.span`, is equivalent since
this amendment: of a `match` of a closure's `catch_unwind` whose `const`
MIR placed at an arm, it makes the call's comment the closure's, above the
`try`, and a comment above that arm prints where the arm's own marker
puts it, as before. The whole corpus compiled with and without it prints
the same, and `lowering-comments-jumps`, `lowering-comments-ends`,
`lowering-unwind-arms` and `js-any-values` pass with it, so it's removed
(2026-11-04). The code keeps the call's own place, which says what the
comment is of.

