// Mutations of crates/rust-js-lower/src/iterators/mir_steps/consumed.rs (ADR
// 0093): who drops what std's code takes by value, the one answer (#2572).
import type { Mutation } from "../../../../mutations";

const file = "crates/rust-js-lower/src/iterators/mir_steps/consumed.rs";

export const mutations: Mutation[] = [
  {
    name: "consumer-source-unclosed",
    breaks:
      "a consumer that takes each item of an owning source, `for_each`, `fold`, is given it unclosed, so a panic in its closure drops none of what's left (#422)",
    file,
    find: "            ConsumedDrop::Closes => self.closing(it, call.iter_ty, span),\n",
    replace: "            ConsumedDrop::Closes => Ok(it),\n",
    tests: ["test/corpus.test.ts", "-t", "^panic_mid_iteration_drops.rs$"],
  },
  {
    name: "consumed-shunt-closed",
    breaks: "a fallible `collect()` into a collection whose own `from_iter` runs code is answered as closed, not its `GenericShunt`'s, so what's left is dropped by both (#1588, #2572)",
    file,
    find: "            return Ok(ConsumedDrop::Shunted);\n",
    replace: "            return Ok(ConsumedDrop::Closes);\n",
    tests: ["test/corpus.test.ts", "-t", "^fallible_collect_into_own_from_iter(_that_panics)?\\.rs$"],
    snapshots: true,
  },
  {
    name: "consumed-generic-refused",
    breaks:
      "generic code that consumes an iterator its caller gives a drop of, `items.into_iter().last()`, takes none, and a caller's destructor is refused",
    file,
    find: "                self.has_generic_drop(receiver)\n                    || (compiled && self.holds_given_drops(receiver))\n",
    replace: "                (compiled && self.holds_given_drops(receiver))\n",
    tests: ["test/corpus.test.ts", "-t", "^extreme_drops.rs$"],
  },
  {
    name: "consumed-held-function-kept",
    breaks:
      "a `map(f)` consumed by `collect` isn't dropped after, nor its function",
    file,
    find: "            .is_some_and(|function| self.drops(function) != crate::drops::Drops::Nothing);\n        // Or an adapter closed",
    replace:
      "            .is_some_and(|function| self.drops(function) != crate::drops::Drops::Nothing && false);\n        // Or an adapter closed",
    tests: ["test/corpus.test.ts", "-t", "^held_function_drops.rs$"],
  },
  {
    name: "consumed-closed-adapter-refused",
    breaks:
      "`iter.take(2).collect()` of a type parameter given its drop is refused, its consumed `Take` not closed",
    file,
    find: "                    || holds_function\n                    || closed\n            }\n",
    replace: "                    || holds_function\n            }\n",
    tests: ["test/corpus.test.ts", "-t", "^collect_of_an_impls_own_associated_type.rs$"],
    snapshots: true,
  },
  {
    name: "compiled-iterator-not-dropped-after",
    breaks:
      "a generic `Peekable` consumed, `p.sum()`'s, isn't dropped after by the drops it's given, so what it holds with a destructor is never dropped",
    file,
    find: "                    || (compiled && self.holds_given_drops(receiver))\n",
    replace: "",
    tests: ["test/corpus.test.ts", "-t", "^peekable_given_back_drops.rs$"],
  },
  {
    name: "compiled-iterator-of-destructors-not-dropped-after",
    breaks:
      "a `Peekable` of a type with a destructor, consumed, isn't dropped after its method",
    file,
    find: "            false => self.is_user_iterator(receiver) || compiled || holds_function,\n",
    replace:
      "            false => self.is_user_iterator(receiver) || holds_function,\n",
    tests: ["test/corpus.test.ts", "-t", "^peekable_given_back_drops.rs$"],
    snapshots: true,
  },
  {
    name: "last-output-holds-its-iterator",
    breaks: "`last()`'s `Option<I::Item>` is taken to hold the `I` it consumed, so std's compiled `Skip::last` never drops its source and its caller's close drops it after the crate's own `last` did (#522)",
    file,
    find: "        let holds = crate::mut_refs::holds_outside_projections(output, receiver);\n",
    replace: "        let holds = output.walk().any(|part| part.as_type() == Some(receiver));\n",
    tests: ["test/std-package.test.ts"],
  },
  {
    name: "extend-taken-to-close",
    breaks: "`v.extend(it)` of an iterator with a `Drop` of items with one is taken to close it, which no `extend` does, so it's never dropped (#1120)",
    file,
    find: "        if !extends\n",
    replace: "        if true\n",
    tests: ["test/corpus.test.ts", "-t", "^iterator_consumed_by_extend_unzip_partition_eq.rs$"],
  },
  {
    name: "extend-by-fold-dropped-after",
    breaks: "an `extend` by `for_each` of the crate's iterator with its own `fold` drops it after, though that `fold` took and dropped it: `set.extend(up())` drops `Up` twice (#1509)",
    file,
    find: "            true => arg_tys.first().is_some_and(|&into| self.extends_by_fold(into)),\n",
    replace: "            true => false,\n",
    tests: ["test/corpus.test.ts", "-t", "^extend_by_fold_drops_once.rs$"],
  },
  {
    name: "own-fold-consumer-dropped-again",
    breaks:
      "an iterator the crate's own fold took and dropped is dropped again after its consumer",
    file,
    find: "            return Ok(ConsumedDrop::OwnFold);\n        }\n        let params_only",
    replace: "        }\n        let params_only",
    tests: ["test/corpus.test.ts", "-t", "^own_fold_through_adapters.rs$"],
  },
  {
    name: "consumed-closed-dropped-again",
    breaks: "an iterator its consumer closes, the crate's with a `Drop` over items with one, is dropped after its `fold`, `for_each`, `count` or `last` too, so twice (#637)",
    file,
    find: "            && (self.consumer_closes(method, receiver, lent, dropping) || self.keyed_consumer_closes(method, arg_tys))\n",
    replace: "            && false\n",
    tests: ["test/corpus.test.ts", "-t", "^(consumed_own_iterator_dropped_once|panic_mid_iteration_drops|panic_mid_fold_drops|adapter_drops|consumers_keep_what_they_built_on_a_panic)\.rs$"],
  },
  {
    name: "keyed-extreme-dropped-after",
    breaks: "`max_by_key` of a key that drops over the crate's iterator, given it closed, drops it after the call too, twice (#789)",
    file,
    find: "            && (self.consumer_closes(method, receiver, lent, dropping) || self.keyed_consumer_closes(method, arg_tys))\n",
    replace: "            && self.consumer_closes(method, receiver, lent, dropping)\n",
    tests: ["test/corpus.test.ts", "-t", "^max_by_key_of_a_dropping_key_over_its_iterator.rs$"],
  },
  {
    name: "try-processed-dropped-after",
    breaks: "what a fallible `collect()` gives the crate's `from_iter` is dropped after it in the build of std's bodies, though that `from_iter` kept or forgot it (#1588)",
    file,
    find: "        if method == Some(IterMethod::Collect) && self.try_processed_into(output, receiver)?.is_some() {\n",
    replace: "        if false {\n",
    tests: ["test/iterator-forms.test.ts", "-t", "fallible_collect_into_own_from_iter with std"],
  },
  {
    name: "fallible-collect-closed-and-dropped-after",
    breaks:
      "generic code's `collect()` into an `Option` or a `Result` of a type parameter's iterator, closed by the collect, is dropped after the call too, twice (#2561)",
    file,
    find: "        ) && self.fallible_collect_closes(output, receiver)\n",
    replace: "        ) && false\n",
    tests: ["test/corpus.test.ts", "-t", "^drops_of_consumed_iterators\.rs$"],
  },
  {
    name: "consumed-fallible-sum-unclosed",
    breaks: "generic code's fallible `sum` or `product` into a `Result` isn't closed as a fallible `collect()` is, so its source is dropped after the call, outside `try_process`'s frame, and the kept `Err` isn't dropped as the source's drop panics (#2580)",
    file,
    find: "            Some(IterMethod::Collect | IterMethod::Sum | IterMethod::Product)\n",
    replace: "            Some(IterMethod::Collect)\n",
    tests: ["test/corpus.test.ts", "-t", "^generic_fallible_(sum_drops_the_kept_err_as_its_source_drop_panics|product_drops_its_source_as_its_step_panics)\\.rs$"],
  },
];
