# 0969. A variable's value of a chain of arms is the chain, every arm its own

Status: Accepted. Records and extends the rule fd6e76df1 made (MIR keeps a
variable's value of three arms a chain) to chains of any length. Amended
below: a chain whose every arm is a plain value is one nested conditional
(the user's decision of 2026-10-27), and a function's value of a chain is
the chain's too (the user's decision of 2026-10-28, rd23).

Case ([0262](0262-when-rust-and-js-disagree.md)): N, no observable
difference: an `if` chain and a conditional give the same value, each test
evaluated in the same order.

## Context

`let x = if a { p } else if b { q } else { r };`, from MIR, is three
branches each assigning `x`. Tidy folds two branches that only give a
value into a conditional, bottom-up, so a chain had its last two folded,
`else { x = b ? q : r; }`, half a chain and half a conditional; fd6e76df1
restored the chain of three, `else if (b) { x = q; } else { x = r; }`, as
a person writes one of more branches.

React's ReactFiberPerformanceTrack picks colors by chains of four arms:

```rust
let color = if yield_duration < 5.0 {
    "primary-light"
} else if yield_duration < 10.0 {
    "primary"
} else if yield_duration < 100.0 {
    "primary-dark"
} else {
    "error"
};
```

It printed `if (yieldDuration < 5) { color = "primary-light"; } else {
color = yieldDuration < 10 ? "primary" : yieldDuration < 100 ? .. : ..;
}`: the restoration took only a conditional of two values, MIR gave the
inner arms through temporaries, `let _7; _7 = ..; color = _7;`, and an
arm MIR wrote the other way round came out negated, `!(selfTime < 100) ?
"error" : ..`.

## Decision

**A variable's value of a chain of arms is the chain, every arm its own**
(`tidy/branches.rs`'s `chained_arms`): the rest folded, `x = b ? q : c ? r
: s`, is `else if (b) { x = q; } else if (c) { x = r; } else { x = s; }`,
the chain going on in each conditional's second value; one of a negated
test, MIR's arms the other way round, `!t ? e : r`, is `if (t) { x = r; }
else { x = e; }`; and one MIR gives through a temporary declared and read
there, `let _7; _7 = v; x = _7;` (`assigned_through`), is `x = v`'s. An
arm's value is a value alone or the conditional of an `if` in that arm,
`wasHydrated ? "tertiary" : "primary"`.

## Alternatives

- **Every arm a conditional, `const color = a ? .. : b ? .. : ..`**, as
  upstream React writes these. The rule for three arms chose the chain;
  which form a chain of values takes is written up for the user.
- **The half-folded form, as before**: two forms in one value.

## Consequences

- Held by `test/lowering-written-branches.test.ts`'s "a variable's value of
  four arms from MIR is their chain" and `test/lowering-branches.test.ts`'s
  three arms; mutations `chain-through-temporary-unseen`,
  `chain-negated-arm-kept`, `chain-rest-one-arm` and `mir-chain-tail-folded`.

## Amendment: a chain of plain values is one nested conditional

The user decided, of React's ReactFiberPerformanceTrack (pt3), between
the chain of statements, every chain of conditionals, and the two by what
the arms are: **a chain giving a variable its value whose every arm is a
plain value, a string or a number literal (`-1` too) or a name, is one
nested conditional**, `const color = yieldDuration < 5 ? "primary-light"
: yieldDuration < 10 ? "primary" : .. : "error";`, as upstream writes its
colors. A chain with an arm that's more, a call, a sum, an object, a
conditional of its own (`wasHydrated ? "tertiary" : "primary"`), stays the
`else if` chain above. `const` comes of it: the variable is given its
value once.

- `tidy/branches.rs`'s `branch_values` asks `value_chains.rs`'s
  `plain_chain_of` first, of an `if` whose arms each give the variable
  one value, one a plain value and the other the rest of the chain MIR
  folded: the arms are those of `chained_arms` (`as_written`, now the one
  place that reads MIR's arms the other way round, `!t ? e : r`, as the
  chain writes them). Not of a temporary's, a `switch`'s arms (ADR 0629),
  or the function's value given by `return`s the program wrote (ADR 0657).
- A `match` is a chain of arms as an `if` chain is, so `match k { Note =>
  "note", Warn { .. } => "warn", Tip(_) => "tip" }` is `const cls = k ===
  "Note" ? "note" : k.TAG === "Warn" ? "warn" : "tip";`.
- Where MIR's chain is in the `if`'s first arm, a nested `match` whose
  `None` comes after its `Some`s, `if (value != null) { align = .. ? .. :
  .. } else { align = "none"; }`, the conditional tests the negation
  first, `value == null ? "none" : value === "Left" ? .. : ..`: the chain
  is in each conditional's second value. ADR 0262 case N: a test and its
  negation are one test, evaluated once, before either value.

Alternatives were the user's options: every chain of three or more arms
the chain (this ADR before), and every chain a conditional, which
Prettier lays across lines however long its arms.

Held by `test/lowering-written-branches.test.ts`'s "four arms" and "three
arms" (moved there from `lowering-branches.test.ts`, at its line cap),
and the guide's `color` and `shade`; mutations `plain-chain-kept`,
`plain-arm-any-value`, `plain-chain-first-arm-any` and
`plain-chain-in-yes-kept`. The corpus' chains of values re-blessed
(`format_options_reached`, `formatter_queries`, `integers_128`, whose
comment before the chain, lost before, is kept, `nested_loop_jumps`,
`writer_callables`, `drop_functions`, `option_falsy_payloads`).
`docs/readability.txt` is blessed with it: no shape's count changed, but
the chains' 36 lines fewer make `renamed x$1`'s same 8,095 a larger share,
696.8 to 697 in 10,000 lines.

## Amendment: a function's value of a chain is the chain's too

The user decided, of React DOM's ReactDOMUnknownPropertyHook (rd23), that
the rule above applies to a function's value as to a variable's: **an
`else if` chain the function returns the value of, every arm a plain
value, is one nested conditional, `return n < 0 ? "negative" : n === 0 ?
"zero" : "positive";`; any arm more, a call, a template, an object, and it
is the whole chain, each arm its `return`**, never half a chain and half a
conditional. Before, `returned_values` folded the chain's last link's two
arms alone, `if (a) { return ..; } else if (b) { return ..; } else {
return c ? "an empty string" : (a template); }`, and left
the rest, the shape the first amendment removed from a variable's value.

Case ([0262](0262-when-rust-and-js-disagree.md)): N, as above: the tests
run in the same order either way, each once.

- `hints.rs` records each `else if`'s own condition (`Written::links`), a
  link of a chain after its head.
- `value_chains.rs`'s `returned_else_ifs`, the pass `chains/returned
  chains`, after `strict switches` has made its `switch`es of chains of
  `js::strict_equals` (ADR 0958), which read the last link folded: of a
  chain the list ends with, of three arms or more, its links after the
  head each a written `else if`, it folds every plain arm into one
  conditional (`plain`, the variable's test), and otherwise gives the last
  link its two arms again, `else if (c) { return ..; } else { return ..;
  }`, in the `else` of the link before where `written_elses` left that a
  guard. Not after a link an `else if let` bound first, `const s = ..; if
  (s != null)`, which ends JS's chain (ADR 0237), its links each an `if` of
  its own: their last two stay one conditional, as before. Taking that rest
  as a chain of its own reads better, but names a temporary `value$1` in
  the corpus' `dyn_any` (a grown `renamed x$1` share), so it waits for the
  user (S/decisions-pending.md, rd23b). Since: the amendment below.
- The fold of two arms stays `returned_values`', which a chain's last link
  takes until `returned chains`: a `switch`'s `default: return s !== c ? 3
  : 4;` of a chain's rest is its conditional of two.
- A `match` that's no `switch` (ADR 0629) isn't an `else if` chain and is
  as before; so is a chain whose last link `conditional` made an `&&` or
  `||` of `bool`s, the user's rule for those.

Alternatives: keep the half fold (two forms in one value); fold only by
`returned_values`' bottom-up walk, which can't know a chain's other arms
when it folds its last link.

Held by `test/lowering-returned-chains.test.ts`; mutations
`returned-chain-plain-unfolded`, `returned-chain-last-link-folded`,
`returned-chain-link-headed`, `returned-chain-guard-unelsed` and
`else-if-links-unrecorded`. Re-blessed: `calc`'s and `combinators`'
snapshots (each the chain), the corpus' `unit_given_to_js` and
`option_bool_tested` (each one conditional) and the guide's `sign`;
`lowering-strict-switches`' `mixed` and `lowering-bindings`' `compare` are
one conditional, `lowering-branches`' `chained` the chain of its arrows.

## Amendment: after an `else if let`'s binding, the rest is a chain of its own (rd23b)

The coordinator applied the user's rule (pt3) to rd23b: the links after
one an `else if let` binds first, `const s = ..; if (s != null) { return
..; }`, are a chain of their own, read as any returned chain is: one nested
conditional of plain arms, `return s != null ? s : n > 5 ? "big" :
"small";`, else the chain as written, `if (s != null) { .. } else if (n >
5) { .. } else { .. }`. Before, its first link was an `if` and its last
two one conditional. Case ([0262](0262-when-rust-and-js-disagree.md)):
None, the same tests in the same order.

`returned_else_ifs` takes such a link for the chain's head where the
statement before it declares what its test reads (`bound`). The corpus'
`dyn_any` now names the `Option`'s Debug arrow's value `value$1`, a
`const` where it was an arrow called in place; on main the readability
baseline's `renamed x$1` share grows 681.4 to 681.5 in 10,000 lines by
that one name, blessed for it: the form it replaces, an arrow called in
place, read worse. Naming that temporary for what it holds, the
downcast's, is the names owner's, written up (S/decisions-pending.md). `catch_unwind_typed_payloads`' last link
is the chain's `else if`. Held by `test/lowering-returned-chains.test.ts`'
"the rest of a returned chain after an else if let's binding is a chain of
its own" and mutation `returned-chain-bound-rest-headless`.

## Amendment: a late `let`'s `if`, as it's written (rd187, the user's decision)

`let n; if flag { n = 1 } else { n = 2 }` was `const n = flag ? 1 : 2;`,
tidy's `branch values` folding any `if` of one assignment each way; since
#121 read React's updateDehydratedSuspenseComponent's binding in place,
its `let error; if (__DEV__ && message) { error = new Error(message); }
else { error = new Error(..); }`, upstream's and the Rust's, was a
conditional too. **An `if` with an `else` the program wrote as a
statement, of type `()`, of a variable it declared, stays that `if`**
(`hints::Written::statement_elses`, `branches::branch_values`); a value's,
`let n = if flag { 1 } else { 2 };`, and a temporary's MIR makes, a
`matches!`'s, are conditionals as before. The user's decision of
2026-10-29, "keep as written". A closure that only sets what it
declares, as `let sign; if (x < 0) { sign = "-"; } ..` does, does
nothing a chain's order shows (`iterators::in_place::does`), so its chain
is an array's still, `xs.map(..)`. Case N: the same value. Held by
`test/lowering-names.test.ts` ("a variable nothing sets again is a
const"), `test/corpus/bindings_before_branch.rs`, the guide's `describe`,
the mutations `statement-if-made-a-conditional` and
`statement-if-of-a-temporary-kept` (`mir/tidy/branches.ts`), and
`closure-own-assignment-does` (`lower/iterators/in_place.ts`).
