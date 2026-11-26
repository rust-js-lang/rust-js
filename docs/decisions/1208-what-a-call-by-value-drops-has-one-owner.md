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
