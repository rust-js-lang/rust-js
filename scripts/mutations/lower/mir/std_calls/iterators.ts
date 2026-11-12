// Mutations of src/lower/mir/std_calls/iterators.rs (ADR 0093).
import type { Mutation } from "../../../../mutations";

export const mutations: Mutation[] = [
  {
    name: "inspect-back-stepped-unmade",
    breaks:
      "an `inspect(f)` the crate steps from its back, held in a variable, isn't made knowing it where it's made: `TypeError: it.back is not a function`",
    file: "src/lower/mir/std_calls/iterators.rs",
    find: "                        IterComb::Inspect,\n",
    replace: "",
    tests: ["test/corpus.test.ts", "-t", "^(inspect_from_back|move_closure_adapter_from_its_back).rs$"],
  },
  {
    name: "combinator-function-undropped",
    breaks:
      "what an `inspect()`'s, a `skip_while()`'s or a `flat_map()`'s function holds with a destructor is never dropped, where Rust drops it with the adapter (ADR 0991)",
    file: "src/lower/mir/std_calls/iterators.rs",
    find: "                        Some(held) => this.closing_function(made, output, held, span)?,\n",
    replace: "                        Some(_) => made,\n",
    tests: ["test/corpus.test.ts", "-t", "^adapter_functions_dropped.rs$"],
  },
  {
    name: "mir-peek-steps",
    breaks: "`p.peek()` of a `Peekable` steps it",
    file: "src/lower/mir/std_calls/iterators.rs",
    find: '                                let [it] = given;\n                                let peeked = Expr::call_helper("$peek", vec![it]);\n',
    replace: '                                let [it] = given;\n                                let peeked = Expr::call_helper("$next", vec![it]);\n',
    tests: ["test/corpus.test.ts", "-t", "iterator_back_ends"],
  },
  {
    name: "mir-next-if-eq-unequal",
    breaks: "`p.next_if_eq(&x)` takes what isn't `x`",
    file: "src/lower/mir/std_calls/iterators.rs",
    find: 'Expr::bin(Op::Eq, Expr::var("item"), x)',
    replace: 'Expr::bin(Op::Ne, Expr::var("item"), x)',
    tests: ["test/corpus.test.ts", "-t", "iterator_back_ends"],
  },
  {
    name: "mir-default-size-hint-one",
    breaks: "std's `size_hint()` of an iterator of the crate's is `(1, None)`",
    file: "src/lower/mir/std_calls/iterators.rs",
    find: "Expr::array(vec![Expr::int(0), Expr::undefined()])",
    replace: "Expr::array(vec![Expr::int(1), Expr::undefined()])",
    tests: ["test/corpus.test.ts", "-t", "size_hint"],
  },
  {
    name: "mir-chars-as-str-refused",
    breaks: "`c.as_str()` of a string's `chars()` from MIR is refused",
    file: "src/lower/mir/std_calls/iterators.rs",
    find: "                if matches!(op, StepOp::AsStr | StepOp::Offset)\n",
    replace: "                if false\n",
    tests: [
      "test/lowering-strings.test.ts",
      "-t",
      "the rest of a string's chars from MIR",
    ],
  },
  {
    name: "mir-stream-refused",
    breaks: "`writeln!(io::stderr(), ..)` from MIR is refused",
    file: "src/lower/mir/std_calls/iterators.rs",
    find: "            Std::Stream(op) => lower(",
    replace: "            Std::Stream(op) if false => lower(",
    tests: ["test/corpus.test.ts", "-t", "std_odds"],
  },
  {
    name: "cycle-owned-items-shared",
    breaks:
      "`cycle()` of an iterator that owns its items gives the same one each time round, where Rust's clones them",
    file: "src/lower/mir/std_calls/iterators.rs",
    find: '                                if owns && this.needs_clone(item) {\n                                    args.push(this.clone_fn("item", item, span)?);\n',
    replace:
      '                                if false && owns && this.needs_clone(item) {\n                                    args.push(this.clone_fn("item", item, span)?);\n',
    tests: ["test/corpus.test.ts", "-t", "^iter_cycle.rs$"],
  },
  {
    name: "partition-dropping-refused",
    breaks:
      "`partition` and `filter_map` of items with a destructor are refused, though they move each on",
    file: "src/lower/mir/std_calls/iterators.rs",
    find: "if dropping.is_some() && !keeps_each && !comb.holds_its_sources() {",
    replace: "if dropping.is_some() && !comb.holds_its_sources() {",
    tests: ["test/corpus.test.ts", "-t", "^picked_items_drop.rs$"],
  },
  {
    name: "zip-other-not-closing",
    breaks:
      "`zip`'s other side that owns items with a destructor isn't closed to drop them",
    file: "src/lower/mir/std_calls/iterators.rs",
    find: "                        Ok(match this.closing_drop(ty, span)? {\n",
    replace: "                        Ok(match None::<Expr> {\n",
    tests: ["test/corpus.test.ts", "-t", "^adapter_drops.rs$"],
  },
  {
    name: "cycle-of-other-refused",
    breaks:
      "`cycle()` of a range, the crate's iterator or a type parameter's is refused",
    file: "src/lower/mir/std_calls/iterators.rs",
    find: "                this.cycled(it, arg_tys[0].peel_refs(), span)?\n",
    replace: '                return Err(this.unsupported(span, "cycle"));\n',
    tests: ["test/corpus.test.ts", "-t", "^cycled_iterators.rs$"],
  },
  {
    name: "cycle-array-drops-allowed",
    breaks:
      "`cycle()` of an array's items with a destructor is allowed, and leaks what it holds",
    file: "src/lower/mir/std_calls/iterators.rs",
    find: '                                    return Err(this\n                                        .unsupported(span, "`cycle()` of an array\'s items with a destructor (#1583)"));\n',
    replace: "",
    tests: ["test/diagnostics.test.ts", "-t", "cycling an array's items"],
  },
  {
    name: "peek-mut-of-numbers-unhandled",
    breaks:
      "`peek_mut` of numbers gives the number, which a write changes nowhere",
    file: "src/lower/mir/std_calls/iterators.rs",
    find: "                                Some(item) if this.mut_ref_form(item).is_cell() => {\n",
    replace: "                                Some(item) if false && this.mut_ref_form(item).is_cell() => {\n",
    tests: ["test/corpus.test.ts", "-t", "^repeat_n_and_peek_mut.rs$"],
  },
  {
    name: "chained-into-iter-raw",
    breaks:
      "what `chain` or `zip` is given is passed as it is, a range's object or an `Option`",
    file: "src/lower/mir/std_calls/iterators.rs",
    find: "                        let e = this.iterable_items(e, ty, span, out)?;\n",
    replace: "",
    tests: ["test/corpus.test.ts", "-t", "^into_iter_arguments.rs$"],
  },
  {
    name: "held-cycle-of-array-generated",
    breaks:
      "a held `Cycle` of an array's iterator is a generator, which keeps neither it cycles",
    file: "src/lower/mir/std_calls/iterators.rs",
    find: "                if this.held_cycle(output).is_some()\n                    || !arg_tys\n",
    replace: "                if !arg_tys\n",
    tests: ["test/corpus.test.ts", "-t", "^held_cycle.rs$"],
  },
  {
    name: "next-if-some-unboxed",
    breaks:
      "`next_if` of items that may look like `None` gives a `None` item as `None` itself, where it's `Some(None)`",
    file: "src/lower/mir/std_calls/iterators.rs",
    find: 'if boxed { "$nextIfSome" } else { "$nextIf" }',
    replace: '"$nextIf"',
    tests: ["test/corpus.test.ts", "-t", "^boxed_option_results.rs$"],
  },
  {
    name: "found-and-then-none-undefined",
    breaks:
      "`and_then` of a place found gives `undefined` where it's `None`, which isn't `-1`, so it's taken for a place",
    file: "src/lower/mir/std_calls/iterators.rs",
    find: "                let none = if mapped { Expr::undefined() } else { Expr::int(-1) };",
    replace:
      "                let none = if mapped { Expr::undefined() } else { Expr::undefined() };",
    tests: ["test/lowering-string-places.test.ts", "-t", "owner stack"],
  },
  {
    name: "chain-array-argument-unstepped",
    breaks:
      "an array given to `chain` is `Iterator.from(xs)`, not a `$iter`, so its `nth` isn't a `Vec`'s and the chain's `skip` steps each (#250)",
    file: "src/lower/mir/std_calls/iterators.rs",
    find: "                                    if this.slice_item(ty.peel_refs()).is_some()\n",
    replace:
      "                                    if false && this.slice_item(ty.peel_refs()).is_some()\n",
    tests: ["test/corpus.test.ts", "-t", "^chain_sides_own_steps.rs$"],
  },
  {
    name: "in-place-flatten-unmarked",
    breaks:
      "a flatten made in place and stepped from its back is taken for std's struct, given its compiled `next_back`, which the recognizers' `flatMap` isn't",
    file: "src/lower/mir/std_calls/iterators.rs",
    find: "                        true => made.in_place(),\n",
    replace: "                        true => made,\n",
    tests: ["test/corpus.test.ts", "-t", "^(flatten_of_iterators_from_back|flatten_from_both_ends_drops_its_inners|own_iterator_flattened_from_its_back|associated_into_iterator_flattened_from_its_back).rs$"],
  },
  {
    name: "filter-map-made-unknowing",
    breaks: "a `filter_map` the crate steps from its back, held in a variable before it's stepped, isn't made knowing it where it's made, and is refused as a library (#473)",
    file: "src/lower/mir/std_calls/iterators.rs",
    find: "                        IterComb::FilterMap,\n",
    replace: "",
    tests: ["test/corpus.test.ts", "-t", "^filter_map_of_given_from_its_back.rs$"],
  },
  {
    name: "chain-stepped-from-back-made-unknowing",
    breaks:
      "a chain the crate steps from its back isn't made knowing its back where it's made: one MIR holds in a variable, of items with a destructor, is refused at its `rev()` (#718)",
    file: "src/lower/mir/std_calls/iterators.rs",
    find: "                        IterComb::Chain,\n",
    replace: "",
    tests: ["test/corpus.test.ts", "-t", "^(rev_made_knowing_its_back|chain_stepped_from_its_back_elsewhere).rs$"],
  },
  {
    name: "peek-mut-generic-unboxed",
    breaks: "`peek_mut` in generic code gives the item, not a cell of it, which generic code writes as a box: JS throws (#1007)",
    file: "src/lower/mir/std_calls/iterators.rs",
    find: "                                Some(item) if this.mut_ref_form(item).is_cell() => {\n",
    replace: "                                Some(item) if this.mut_ref_form(item).is_cell() && !this.is_unknown(item) => {\n",
    tests: ["test/corpus.test.ts", "-t", "^generic_mut_lent_by_std.rs$"],
  },
  {
    name: "leaves-none-source-unclosed",
    breaks:
      "an owning iterator given whole to `partition`, `max_by` or `max_by_key` isn't closed, so what's left when its callback panics is never dropped (#423)",
    file: "src/lower/mir/std_calls/iterators.rs",
    find: "                        false if keeps_each => {\n",
    replace: "                        false if false => {\n",
    tests: ["test/corpus.test.ts", "-t", "^panic_part_way_drops.rs$"],
  },
  {
    name: "combinator-consumer-closes-unasked",
    breaks: "`it.partition(f)` of the crate's iterator with a `Drop` closes it as it ends, where `consumed_call` drops it after, so it's dropped twice (#1120)",
    file: "src/lower/mir/std_calls/iterators.rs",
    find: "                            this.consumer_closing(it, (method, iter_ty, false), dropping.is_some(), span)?\n",
    replace: "                            this.closing(it, iter_ty, span)?\n",
    tests: ["test/corpus.test.ts", "-t", "^iterator_consumed_by_extend_unzip_partition_eq.rs$"],
  },
  {
    name: "peeked-object-record-dropped",
    breaks: "`peek()`'s object is let through but not recorded as stored in the `Peekable`'s `peeked`, its address an allocation of its own (#1475)",
    file: "src/lower/mir/std_calls/iterators.rs",
    find: "                                    Some(record) => record(peeked),\n",
    replace: "                                    Some(_) => peeked,\n",
    tests: ["test/corpus.test.ts","-t","^address_of_an_object_std_lends_from_its_storage.rs$"],
  },
  {
    name: "cycle-of-dropping-items-refusal-unnamed",
    breaks: "the refusal of `cycle()` of an array's items with a destructor doesn't name #1583, which tracks it",
    file: "src/lower/mir/std_calls/iterators.rs",
    find: "with a destructor (#1583)\"));\n",
    replace: "with a destructor\"));\n",
    tests: ["test/diagnostics.test.ts", "-t", "cycling an array's items with a destructor"],
  },
];
