# 1207. The runtime says which of its helpers give an iterator

Status: accepted (2026-11-25, #2229)

## Context

`is_js_iterator` (iterators.rs) decides whether a value is already a JS
iterator, so that a reader writes no `Iterator.from` around it. For a
runtime helper's call, it answered from `ITERATOR_HELPERS`, a list of 62
names kept in lowering. runtime-package.test held that list to its own copy.

The list missed `$lent` and `$held`, which always give one. So
`$lent(src).map(f)` was wrapped as `Iterator.from($lent(src).map(f))`, for
nothing (#2229). The same blind spot made `counted_where_kept` write a
`$iter(..)` of one, which threw (#2226). The list was a second catalog of
what the runtime makes, outside the runtime.

## Decision

Each helper's own source says whether a call of it gives a JS iterator:

- a generator says so by its header, `function*`;
- any other function says so by a mark on the line directly above its
  declaration: `// gives: an iterator` (`runtime::GIVES_AN_ITERATOR`).

The helpers carry no typed return, neither a JSDoc `@returns` nor a
`.d.ts` declaration, so the mark is a comment of its own. If they ever
gain typed returns, read the answer from the type instead.

`runtime/signatures.rs`, the one reader of the helpers' declarations (ADR
1186), reads the mark with the rest. It refuses a mark that's followed by
no helper's declaration, or that sits on a generator. `runtime::gives_iterator(name)`
is the one answer. `is_js_iterator` asks it of a call's helper, through
`helper_name`, the representation's API. `ITERATOR_HELPERS` is deleted;
test/architecture-iterators.test.ts holds that no list returns. A unit test
in `signatures.rs` holds the answer to the helpers it gives: 64 names, the
list's 62 plus `$lent` and `$held`.

**Why not a fact?** A `Known` mark set where the call is made would also
end the list. #2229's shadow tried one, set in `Expr::call`. Over the corpus
it lost the mark at 3 reads: a pass rebuilds a `$mapEnds(..)` call without
its facts (#2602). A false "not an iterator" there would decide a drop path
(`std_calls/iterators.rs`), so the answer is read from the call's helper,
which no rebuild loses.

JS's own builtins stay as JS-semantics rules in `is_js_iterator`:
`Iterator.from`, `values()`, `keys()` and `entries()`, and adapters of one.
Making those come from their makers is a follow-up.

## Consequences

Which ADR 0262 case this is: none. An observable difference? No: the
`Iterator.from` it no longer writes changed nothing a program sees. Over the
corpus (3,078 cases), the crate pairs and every iterator form, one snapshot
moved, `capacity_kept_of_a_lent_adapter.js`: two
`Iterator.from($lent(src).map(..))` became `$lent(src).map(..)`.
`std/runtime.js` carries the 61 marks.

A new helper that gives an iterator says so in its own file, with the mark
or as a generator; nothing else changes. Mutation `iterator-mark-unread`.
