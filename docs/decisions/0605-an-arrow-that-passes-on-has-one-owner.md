# 0605. Whether an arrow only passes on what it's given has one owner

Status: Accepted. Amends [0198](0198-optional-handlers.md) and
[0279](0279-callbacks-passed-on.md). Amended below: `passed_on.rs` owns
the question with each position's rule, and a binding's `FnOnce` callback
is one ([0262](0262-when-rust-and-js-disagree.md), amended).

Case ([0262](0262-when-rust-and-js-disagree.md)): A: the output is
unchanged.

## Context

Two places asked one question, whether an arrow is just the function it
calls with its parameters:

- an array method's callback, `xs.map((x) => f(x))` as `xs.map(f)`
  (ADR 0279), `iterators::passed_on`, of an arrow that returns the call;
- an event's handler, `onClick={() => onClear()}` as `onClick={onClear}`
  (ADR 0198), `jsx.rs`'s own check, of a call of nothing, given back or
  not.

Each was right where it was, and they'd drift: found porting React's
scheduler, whose `onmessage = performWorkUntilDeadline` neither covers.

## Decision

**`iterators::passed_on` answers it, for both**: an arrow that passes on
what it's given, in order, to a function it doesn't name a parameter;
and, where nothing reads what the arrow gives, a handler's, one whose
call is a statement, `() => { f(); }`. Each position still says whether
it may: an array method's, where the arrow calls the crate's own (ADR
0279); an event's, where React ignores what it gives (ADR 0198).

Where else a closure may be its function, `onmessage = ..`, isn't decided
here: React's rules forbid some, `useEffect(onLog)` of a `useEffectEvent`.

## What holds it

- `test/architecture.test.ts`, "an arrow that passes on what it's given
  is passed_on's alone".
- ADRs 0198's and 0279's tests, unchanged; their mutations, moved to the
  owner.

## Amendment: the owner takes the position

`iterators::passed_on` answered the shape, but each position kept its own
rules beside it: jsx's handler had a second shape check, of an optional
handler, `(e) => { if (f != null) { f(e); } }`, and asked THIR whether
the call was Rust's (`calls_rust`), where an array method's read MIR's
spans, behind its own list of method names. Seven more functions checked
the shape of the arrows rust-js makes of a function as a value, each its
own way: a dictionary's `fmt`, a binding's `fn`, a trait method's and any
function's value, an impl's dictionary entry, a function's drops, and
`js::called_alone`; and preparation's forwarding function (ADR 0610).
Found deciding a binding's `FnOnce` callback, which would have been a
third position rule beside them, and a wrong answer they hid: `xs.map(|x|
many(&[x]))` of a function of rest arguments was `xs.map(many)`, which
counts the index and the array.

**`src/lower/passed_on.rs` owns it: `passed_on(f, taker, forwarding)`**,
each position a `Taker`, its rule there:

| Taker | Where | It may be the function of |
|---|---|---|
| `ArrayMethod` | `Taker::array_method(name)`, `map`, `filter` and the rest (ADR 0279) | the crate's function or a closure, or a generic `impl Fn` (case B) |
| `Handler` | `Taker::prop(name)`, `on` and a capital (ADR 0198) | any of the closure's, a `dyn Fn` prop's too; a statement of the call; an optional handler passed on |
| `OnceCallback` | `Taker::parameter(tcx, def_id, i)`, a binding's `FnOnce` parameter (ADR 0262, amended) | the crate's function or a closure; a statement of a call of `()` |
| `Made` | an arrow rust-js makes, which what's given it calls with its parameters alone | a function, a namespace's member too |
| `FnValue` | a function as a value, `.map(str::len)` (ADR 0538) | a function not a runtime helper, or an arrow of as many parameters |

- **What a closure passes its call on to is one fact, MIR's**: where MIR
  makes a closure of one call, `forwarded_to` says whether it calls the
  crate's function or a closure (`Own`), a generic `impl Fn` (`Generic`)
  or a `dyn Fn` (`Dyn`), and `mir.rs`'s `forwarding` keeps it by where
  it's written; every taker reads it. A binding's call has none, nor a
  function of rest arguments, which reads all it's given: the arity check
  the user asked for, where JS gives the function more than the closure
  did.
- **The shape is `js::passes_on`'s**, of a function's parameters and its
  body, rest arguments spread on too, which preparation's forwarding
  function asks as well: owned output can't ask lowering.
- **Each caller is a thin wrapper**: it says where the arrow is given and
  takes what `passed_on` gives.

### What holds it

- `test/architecture.test.ts`, "an arrow that passes on what it's given
  is passed_on's alone": the shape is matched in `js/stmt_walk.rs` alone,
  an optional handler's in `passed_on.rs` alone, the removed functions
  stay gone, the callers are the ones listed, and `forwarded_to` is asked
  where MIR makes a closure alone.
- `test/lowering-passed-on.test.ts`, and the corpus's
  `once_callback_passed_on.rs`, against native Rust: a once-callback of a
  function of the crate's is it, and a listener's, a JS function's, a
  rest function's and an array method's of a `dyn Fn` keep their arrows.
- `scripts/mutations/lower/passed_on.ts`, with the mutations moved there
  from `jsx.rs`, `iterators/eager.rs`, `iterators/ends.rs` and
  `mir/tidy/values.rs`.
