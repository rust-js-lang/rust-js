// Mutations of src/lower/iterators/compiled_steps.rs (ADR 0093).
import type { Mutation } from "../../../mutations";

export const mutations: Mutation[] = [
  {
    name: "iterator-evidence-of-any-param",
    breaks:
      "a parameter bound by a trait `Iterator` is a supertrait of is stepped through that trait's dictionary, which has no `Iterator`",
    file: "src/lower/iterators/compiled_steps.rs",
    find: "        if !self.iterator_given_as_it_is(ty) {\n",
    replace: "        if false {\n",
    tests: ["test/corpus.test.ts", "-t", "^(associated_iterator_dropped|associated_iterator_given_as_it_is|into_iter_of_ref_param|generic_associated_types|mut_kept_by_what_a_call_gives_back).rs$"],
  },
  {
    name: "std-iterator-hint-missing",
    breaks:
      "a compiled std iterator's struct is made with no `size_hint` of its own, a `Fuse`'s, so `$sizeHint` of it is a lazy iterator's",
    file: "src/lower/iterators/compiled_steps.rs",
    find: '            true => Some(self.user_next(ty, iterator, "size_hint", span)?.0),\n',
    replace: "            true => None,\n",
    tests: ["test/std-package.test.ts"],
  },
  {
    name: "own-size-hint-dropped",
    breaks:
      "the crate's iterator made a JS iterator isn't given its own `size_hint`, so generic code asks `(0, None)`",
    file: "src/lower/iterators/compiled_steps.rs",
    find: "        let (next, boxed) = self.user_next(ty, trait_id, method, span)?;\n        let hint = self.own_size_hint(ty, span)?;\n",
    replace:
      "        let (next, boxed) = self.user_next(ty, trait_id, method, span)?;\n        let hint = None;\n",
    tests: ["test/corpus.test.ts", "-t", "^adapter_size_hints.rs$"],
  },
  {
    name: "peekable-hint-undictionaried",
    breaks:
      "a compiled `Peekable`'s hint isn't given its source's dictionary, so it says `(0, None)` of the crate's iterator",
    file: "src/lower/iterators/compiled_steps.rs",
    find: "                Some(match self.iterator_evidence(source, iterator) {\n",
    replace: "                Some(match None::<Expr> {\n",
    tests: ["test/std-package.test.ts", "-t", "the committed @rust-js/std"],
  },
  {
    name: "user-iterator-backless",
    breaks:
      "the crate's double-ended iterator, asked its back, is made without its `next_back`",
    file: "src/lower/iterators/compiled_steps.rs",
    find: '            Some(double_ended) if back => Some(self.user_next(ty, double_ended, "next_back", span)?.0),\n',
    replace: "            Some(_) if back => None,\n",
    tests: ["test/corpus.test.ts", "-t", "^user_iterator_ends.rs$"],
  },
  {
    name: "compiled-last-unhooked",
    breaks:
      "a fuse's or a peekable's compiled `last` isn't given to its JS iterator",
    file: "src/lower/iterators/compiled_steps.rs",
    find: '        if self.compiled_step(ty, iterator, "last") {\n',
    replace: "        if false {\n",
    tests: ["test/std-package.test.ts"],
  },
  {
    name: "user-iterator-reversed-forward-own",
    breaks:
      "`rev()` of the crate's iterator is given its forward `nth` and `fold` (#217)",
    file: "src/lower/iterators/compiled_steps.rs",
    find: '        let own = self.own_methods(ty, method == "next_back", span)?;\n',
    replace: "        let own = self.own_methods(ty, false, span)?;\n",
    tests: ["test/corpus.test.ts", "-t", "^rev_own_back_methods.rs$"],
  },
  {
    name: "user-iterator-knowing-closed-rewrapped",
    breaks:
      "the crate's iterator closed to drop it, made knowing its back, is wrapped as if it were the crate's value, whose methods it's then given: a wrong value (#223)",
    file: "src/lower/iterators/compiled_steps.rs",
    find: "        if let Some((it, drop)) = crate::lower::iterators::closed_parts(&value) {\n",
    replace:
      "        if let Some((it, drop)) = crate::lower::iterators::closed_parts(&value).filter(|_| false) {\n",
    tests: ["test/corpus.test.ts", "-t", "^own_iterator_dropped_from_back.rs$"],
  },
  {
    name: "user-iterator-knowing-lent-rewrapped",
    breaks:
      "a `&mut` of the crate's iterator, lent, made knowing its back, is wrapped as if it were the crate's value: a wrong value (#225)",
    file: "src/lower/iterators/compiled_steps.rs",
    find: "        if is_lent(&value) {\n",
    replace: "        if false {\n",
    tests: [
      "test/corpus.test.ts",
      "-t",
      "^own_iterator_dropped_through_adapters.rs$",
    ],
  },
  {
    name: "user-iterator-knowing-lent-folds",
    breaks:
      "a `&mut` of the crate's iterator, lent, is given its own folds, which take it and drop it (#225)",
    file: "src/lower/iterators/compiled_steps.rs",
    find: "            return self.user_iterator_knowing(it, borrowed, (back, len), span);\n",
    replace:
      "            return self.user_iterator_knowing(it, ty, (back, len), span);\n",
    tests: [
      "test/corpus.test.ts",
      "-t",
      "^own_iterator_dropped_through_adapters.rs$",
    ],
  },
  {
    name: "compiled-rev-closes-nothing",
    breaks: "a held `Rev` closed, or dropped, drops nothing it holds",
    file: "src/lower/iterators/compiled_steps.rs",
    find: "            if !body.is_empty() {\n                props.push(crate::js::Prop::Field(\n",
    replace:
      "            if false {\n                props.push(crate::js::Prop::Field(\n",
    tests: ["test/std-package.test.ts", "-t", "the committed @rust-js/std"],
  },
  {
    name: "compiled-rfold-unasked",
    breaks:
      "a held `Map` given to a compiled `fuse()` is folded from its back by each `next_back`, not its own `rfold`, which the crate's own passes on",
    file: "src/lower/iterators/compiled_steps.rs",
    find: "        Ok(compiled.then_some(found))\n",
    replace:
      '        Ok(compiled.then_some(found).filter(|_| method != "rfold"))\n',
    snapshots: true,
    tests: ["test/corpus.test.ts", "-t", "^(own_back_through_adapters).rs$"],
  },

  {
    name: "reversed-held-made-again",
    breaks:
      "a variable's `rev()` of the crate's iterator, whose form isn't there, is made again as if it were the iterator itself: a wrong answer, not refused",
    file: "src/lower/iterators/compiled_steps.rs",
    find: "        let Some(value) = crate::lower::iterators::steps(&value).cloned() else {\n",
    replace:
      "        let Some(value) = crate::lower::iterators::steps(&value).cloned().or(Some(value)) else {\n",
    tests: ["test/diagnostics.test.ts", "-t", "held rev of the crate"],
  },
  {
    name: "reversed-own-folds-unswapped",
    breaks:
      "an in-place `rev()` of the crate's iterator made knowing its back folds from its back by the iterator's own `fold`, not its `rfold`",
    file: "src/lower/iterators/compiled_steps.rs",
    find: "            fold: forward.rfold,\n",
    replace: "            fold: None,\n",
    snapshots: true,
    tests: [
      "test/corpus.test.ts",
      "-t",
      "^(rev_in_place_through_adapters|own_back_through_adapters).rs$",
    ],
  },
  {
    name: "compiled-fold-untaken",
    breaks:
      "std's compiled `fold` of a `fuse()` isn't said to take it, so `$closing` drops what it held again after (ADR 0912)",
    file: "src/lower/iterators/compiled_steps.rs",
    find: '            takes.push("fold");\n',
    replace: "",
    tests: [
      "test/std-package.test.ts",
      "-t",
      "the committed @rust-js/std is std's bodies",
    ],
  },
  {
    name: "compiled-count-unlisted",
    breaks:
      "a held `enumerate()`'s `count` isn't std's compiled one, its source's own, so it steps each item (ADR 0912)",
    file: "src/lower/iterators/compiled_steps.rs",
    find: '        if self.compiled_step(ty, iterator, "count") {\n            let (count, _) = self.user_next(ty, iterator, "count", span)?;\n',
    replace:
      '        if false {\n            let (count, _) = self.user_next(ty, iterator, "count", span)?;\n',
    tests: [
      "test/std-package.test.ts",
      "-t",
      "the committed @rust-js/std is std's bodies",
    ],
  },
  {
    name: "compiled-try-fold-unlisted",
    breaks:
      "std's struct has no compiled `try_fold`, so std's default `nth` through it asks a `skip`'s source `nth(n)` where Rust's asks `nth(n - 1)` (ADR 0913)",
    file: "src/lower/iterators/compiled_steps.rs",
    find: '        if let Some(try_fold) = self.compiled_try_fold(ty, (iterator, "try_fold"), span)? {\n',
    replace: "        if let Some(try_fold) = None::<Expr> {\n",
    tests: [
      "test/std-package.test.ts",
      "-t",
      "the committed @rust-js/std is std's bodies",
    ],
  },
  {
    name: "compiled-fold-given-no-unwind",
    breaks:
      "a compiled std fold, a `Fuse`'s, is given no drop of its accumulator, so a stage's panic inside it leaves it undropped (#424)",
    file: "src/lower/iterators/compiled_steps.rs",
    find: "        let dropped = vec![(acc, Expr::var(unwind)), (acc + 1, Expr::var(ends))];\n",
    replace:
      "        let dropped = vec![(acc + 1, Expr::var(ends))];\n",
    tests: [
      "test/std-package.test.ts",
      "-t",
      "the committed @rust-js/std is std",
    ],
  },
  {
    name: "compiled-try-fold-given-no-unwind",
    breaks:
      "std's compiled `try_fold` of its struct, `TakeWhile`'s, which its `fold` is, is given no drop of its accumulator: a stage's panic leaves it undropped (#423)",
    file: "src/lower/iterators/compiled_steps.rs",
    find: '                drops: &[(acc, Expr::var(unwind)), (acc + 1, Expr::var(ends))],\n',
    replace: '                drops: &[(acc + 1, Expr::var(ends))],\n',
    tests: [
      "test/std-package.test.ts",
      "test/iterator-forms.test.ts",
      "-t",
      "the committed @rust-js/std is std|^generic_folds_unwind with",
    ],
  },
  {
    name: "compiled-last-not-taken",
    breaks: "std's compiled `last` doesn't take its struct whole, so closing it after drops its source again (#522)",
    file: "src/lower/iterators/compiled_steps.rs",
    find: "            takes.push(\"last\");\n",
    replace: "",
    tests: ["test/std-package.test.ts"],
  },
  {
    name: "peekable-default-len-unhinted",
    breaks: "a `Peekable`'s `len`, std's default of its `size_hint`, isn't given its struct, whose hint is `$peekableHint`, so a `skip()` of it stepped from its back reads a length it doesn't have and throws (#706)",
    file: "src/lower/iterators/compiled_steps.rs",
    find: "        (exact && self.hinted_by_its_struct(ty)).then(|| {\n",
    replace: "        (exact && self.compiled_step(ty, std_item(self.tcx, StdItem::Iterator), \"size_hint\")).then(|| {\n",
    tests: ["test/corpus.test.ts", "-t", "^skip_of_a_peekable_from_back.rs$"],
  },
  {
    name: "compiled-fold-f-undropped",
    breaks: "std's compiled fold isn't given its `F`'s drop, what drops with `f`, so it never drops it before what it has left (#710)",
    file: "src/lower/iterators/compiled_steps.rs",
    find: "        let dropped = vec![(acc, Expr::var(unwind)), (acc + 1, Expr::var(ends))];\n",
    replace: "        let dropped = vec![(acc, Expr::var(unwind))];\n",
    tests: ["test/std-package.test.ts"],
  },
  {
    name: "compiled-try-fold-given-no-ends",
    breaks: "std's compiled `try_fold` of its struct, `SkipWhile`'s, is given no drop of its `F`, so a panic in its first step drops the accumulator first (#768)",
    file: "src/lower/iterators/compiled_steps.rs",
    find: '                drops: &[(acc, Expr::var(unwind)), (acc + 1, Expr::var(ends))],\n',
    replace: '                drops: &[(acc, Expr::var(unwind))],\n',
    tests: [
      "test/std-package.test.ts",
      "test/iterator-forms.test.ts",
      "-t",
      "the committed @rust-js/std is std|^skip_while_first_step_unwinds with",
    ],
  },


  {
    name: "source-iter-steps-unwritten",
    breaks:
      "std's compiled structs are made with no `inner`, their `SourceIter::as_inner`'s step, so a collect in place guards a `map`'s whole close, its function's drop too (#852)",
    file: "src/lower/iterators/compiled_steps.rs",
    find: "        props.extend(self.source_iter_steps(ty, span)?);\n",
    replace: "",
    tests: ["test/std-package.test.ts", "-t", "committed @rust-js/std is std's bodies"],
  },
  {
    name: "box-param-evidence-unmade",
    breaks: "generic code's `Box<I>` of an `I` held as it is has no dictionary, so it's stepped as a JS iterator its caller's value isn't (#1611)",
    file: "src/lower/iterators/compiled_steps.rs",
    find: "        if let Some(inner) = ty.boxed_ty() {\n",
    replace: "        if let Some(inner) = ty.boxed_ty().filter(|_| false) {\n",
    tests: ["test/corpus.test.ts", "-t", "^boxed_iterator_steps_as_std_box_does.rs$"],
  },
];
