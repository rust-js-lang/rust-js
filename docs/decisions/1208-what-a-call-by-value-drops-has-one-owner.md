# 1208. What a call by value drops has one owner

Date: 2026-11-26.

Status: accepted (#2600, its steps 1–3; step 4, lifting the refused
cases, to follow).

## Context

"What does calling `F` by value, `<F as FnOnce<Args>>::call_once`, drop
of it?" was answered in several places, each testing the closure's kind
its own way:

- `recognition::function_kind`, a closure's kind or a type parameter's
  strongest `Fn` bound, read by analysis/closures.rs, its `called.rs`,
  combinators/callbacks.rs, traits/dyns.rs and passed_on.rs;
- callbacks.rs' `boxed_function_kind`, a box's;
- `recognition::is_fn_once_dyn` and dyns.rs' own reading of a `dyn`'s
  trait, beside called.rs' and instances.rs' readings of a `dyn Fn`;
- `args.as_closure().kind() != FnOnce` at six sites in analysis/closures.rs,
  `called.rs`, instances.rs and drops/facts.rs;
- mir/elaborated.rs, reading rustc's `ClosureOnce` shim to compare drops.

The answer is small and its type alone decides it. A `Fn` or `FnMut`
closure is dropped whole by rustc's `ClosureOnce` shim, where the call
returns and where it unwinds. An `FnOnce` closure drops what its own body
drops. `Box<F>`'s moves out what it holds, calls it, and frees the box.
`&F` and a function pointer drop nothing. Every site that read a kind
collapsed `Fn` and `FnMut` into one answer.

Most of the sites don't decide *what* is dropped. They decide *how* the
JS gets the drop run:

- **(A)** a closure's body is written as its `call_once`: analysis/
  closures.rs' `called_once` and `moved_once`, read by mir/closures.rs
  and drops/facts.rs;
- **(B)** a wrap where a call by value takes a `Fn`: dyns.rs'
  `$droppingOnce`, callbacks.rs' `boxes_called_once` and
  `handled_argument`;
- **(C)** a refusal: drops/facts.rs' two refusals.

## Decision

**The owner is `drops/call_once.rs`.** `OnceDrop` is its answer:
`Whole`, a `Fn`'s or `FnMut`'s, which what holds it drops, and its
`call_once` as that call returns or unwinds; or `Body`, an `FnOnce`'s,
whose body drops what it took. It is asked by type:

- `call_once_drop(tcx, owner, ty)`: a closure's by its kind
  (`OnceDrop::of_closure`); a type parameter's or an associated type's by
  the strongest `Fn` bound `owner` gives it, or a binding's union's
  `Function` member (ADR 1020); an `AssertUnwindSafe`'s as what it holds
  (ADR 0660);
- `dyn_call_once_drop(tcx, predicates)`: a `dyn Fn*`'s by its trait;
- `boxed_call_once_drop(tcx, ty)`: of a box, what its `call_once` drops of
  the function it moves out (#1220);
- `closure_once_shim(tcx, closure, args)`: rustc's `ClosureOnce` shim
  itself, whose drops of `_1` are `Whole`'s. mir/elaborated.rs' drops
  shadow reads it to hold rust-js's body-as-`call_once` to rustc's.

**(B) asks the owner.** Each site's own kind test is deleted, with no
fallback: dyns.rs' `once`, callbacks.rs' `called`, `handled_argument`,
`drops_unrun` and `boxes_called_once`, passed_on.rs' once-callback.

**(A) is one predicate,** `ClosureFacts::body_is_its_call_once(closure,
holds_a_drop)`: a closure whose call drops it whole, written as its
`call_once` where every call of it is by value. That is one a binding
calls once, one given back where JS calls it once, or, where it holds
what drops, one moved once into an `F: FnOnce`. mir/closures.rs and
drops/facts.rs both ask it. `called_once` and `moved_once` are no longer
asked apart. The sets that feed it, and called.rs' and instances.rs'
reading of who calls what by value, ask `OnceDrop::of_closure` and
`dyn_call_once_drop` in place of their own kind tests. ClosureFacts keeps
only the *who*.

Out of scope: mir/shims.rs' `Through`, which says which trait a call goes
through, a different question; `generic_facts::called_through`, its
input; and `writes_what_it_took`, which asks whether a closure writes
what it took, for its clone.

## Consequences

Byte-identical: the output of every corpus case, crate pair and iterator
form is unchanged. Step 4 lifts the 12 refused cases through
`instances_of`, where every call of a closure held in a struct, an
`Option` or a `Vec` is by value; their moves come apart.

The catalog of mutations follows the code: `recognition/traits.rs`' four
of `function_kind` move to `drops/call_once.ts`, which adds
`once-drop-of-fn-mut-its-body` and `boxed-dyn-call-once-unread`;
`moved-once-body-not-its-call-once` holds the predicate; the consumers'
mutations follow their lines.

## Amendment: step 4, closures moved into what's made (#2600)

The refusals failed on the *who*. `moved_once` saw a `let`'s closure only
where its variable was a call's argument. THIR writes a struct's, an enum
variant's, a tuple's and an array's making as an aggregate, not a call,
so `Once(h)`, `Some(h)` and `W { o: Once(h) }` were never moves. A
closure made where it's given to a call, `forced(move || ..)`, wasn't
counted either. `moved_once` now counts both. Each is kept, as before,
only where `Called` says a call by value calls it and none by `&`,
through every generic caller and impl `instances_of` follows. Its body is
then its `call_once`, and what it took is dropped as that call ends or
unwinds. One never called is dropped with what holds it, once.

Six refusals lift, each native-equal and renamed off `_refused`:
`fn_closure_moved_into_a_struct_called_once`,
`fn_closure_in_a_nested_struct_called_once`,
`fn_closure_in_a_struct_a_function_calls_once`,
`fn_closure_run_by_a_guards_drop`, `fn_once_through_another_bound` and
`fn_closure_through_a_blanket_impl`. A seventh shape lifts with them: the
diagnostics case of a `relay<F: Fn>` that moves `f` into an `F: FnOnce`
without calling it. It now reads `f() + once(f)`, which stays refused.

Two stay refused by design: `fn_closure_called_then_moved_into_a_once_callee_refused`
and `fn_closure_given_two_bounds_deep_refused` each call the closure by
`&` before the call by value. A body that drops what it took at its end
would drop it at the first call. New cases:
`fn_closures_moved_into_what_is_made_called_once` (a tuple, an array, an
`Option`, one left uncalled, one dropped beside a called one, a panic) and
`fn_closure_in_a_struct_called_by_ref_then_by_value_refused`.

Its arm for a closure made where it's given to a call answers ADR 1020's
question too: a binding whose callback JS calls once takes a type
parameter bound by `FnOnce` (or a union whose `Function` member is), which
`Called` reads as a call by value. So `called_once_by_js`, which answered
it apart from `Taker::OnceCallback`, became a second authority. Its
mutation survived over the whole corpus. It's deleted, and `called_once`
holds only `returned_once`'s `Fn` closures (#1881). A closure given to a
binding that drops nothing keeps its own body, as one moved once does.
Its two mutations went with it (`called-once-closures-unseen`,
`called-once-of-fn-once-closures`), and `returned-fn-closure-not-call-once`
moved to the new line.

The four boxed cases, a `Box<dyn Fn>` held in what's given, are #2610's.
Lifting them needs a fact of its own, whether every call of a `dyn` type
is by value.

The metamorphic `library` rewrite of the first lifted case found #2609,
on main already, which landed first.

Mutations: `moved-once-into-a-struct-unseen`,
`moved-once-into-a-tuple-unseen` and `moved-once-made-in-place-unseen`
(analysis/closures.ts). `closure-moved-into-a-once-type-unchecked` is
retargeted to the new refused case. instances.ts' three are retargeted
to the lifted cases.
