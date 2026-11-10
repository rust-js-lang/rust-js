// Mutations of src/lower/std_impls/clones.rs (ADR 0093).
import type { Mutation } from "../../../mutations";

export const mutations: Mutation[] = [
  {
    name: "map-keys-cloned-by-value-alone",
    breaks:
      "a clone of a B-tree of keys found by their own `cmp`, or a caller's class, shares its keys and never calls their `Clone`, as it did before #924",
    file: "src/lower/std_impls/clones.rs",
    find: "        let key = Some(key_ty).filter(|&k| self.needs_clone(k));\n",
    replace: "        let key = Some(key_ty).filter(|&k| self.is_value_key(k) && self.needs_clone(k));\n",
    tests: ["test/corpus.test.ts", "-t", "^btree_clone_own_keys.rs$"],
  },
  {
    name: "map-keys-each-cloned",
    breaks:
      "a clone of a map clones each key, a primitive's or one whose clone nothing tells from it too, `[{ ...key }, value]`, where the key itself was its clone (#924)",
    file: "src/lower/std_impls/clones.rs",
    find: "        let key = Some(key_ty).filter(|&k| self.needs_clone(k));\n",
    replace: "        let key = Some(key_ty).filter(|&k| !k.is_primitive() || self.needs_clone(k));\n",
    tests: ["test/corpus.test.ts", "-t", "^btree_own_ord.rs$"],
    snapshots: true,
  },
  {
    name: "btree-clone-searched",
    breaks:
      "a clone of a B-tree of keys found by their own `cmp`, a sorted map's of a quiet order, is made by searching for each, asking `cmp`, where std's `clone_subtree` asks none (#924)",
    file: "src/lower/std_impls/clones.rs",
    find: "                let in_order = sorted && self.found_by_cmp(key_ty) || nodes;\n",
    replace: "                let in_order = false && self.found_by_cmp(key_ty) || nodes;\n",
    tests: ["test/corpus.test.ts", "-t", "^(btree_clone_own_keys|btree_own_ord).rs$"],
    snapshots: true,
  },
  {
    name: "uncounted-weak-cloned",
    breaks:
      "a clone of an uncounted `Weak` copies what it points at, or is refused, where it's the same object (ADR 0843)",
    file: "src/lower/std_impls/clones.rs",
    find: "        if self.uncounted_weak(ty).is_some() {\n            return false;\n        }\n",
    replace: "",
    tests: ["test/corpus.test.ts", "-t", "^weak_uncounted_uses.rs$"],
  },
  {
    name: "value-key-clone-shares-keys",
    breaks:
      "a map's clone shares its keys, which a consumed original's owner then changes",
    file: "src/lower/std_impls/clones.rs",
    find: "        let key = Some(key_ty).filter(|&k| self.needs_clone(k));\n",
    replace: "        let key = Some(key_ty).filter(|&k| false && self.needs_clone(k));\n",
    tests: ["test/corpus.test.ts", "-t", "value_keys"],
  },
  {
    name: "sender-clone-uncounted",
    breaks:
      "a sender's clone is the same sender, not one more, so dropping both drops one too many",
    file: "src/lower/std_impls/clones.rs",
    find: '            return Ok(Expr::call(Expr::helper("$cloneSender"), vec![place]));',
    replace: "            return Ok(place);",
    tests: ["test/corpus.test.ts", "-t", "channels"],
  },
  {
    name: "untagged-clone-shared",
    breaks:
      "a clone of an untagged enum's `Vec` is the same array, which a push to the first changes",
    file: "src/lower/std_impls/clones.rs",
    find: "            ty::Adt(adt, args) if self.untagged(ty).is_some() => {\n                let itself = self.untagged_changed_itself(ty);\n",
    replace:
      "            ty::Adt(adt, args) if false => {\n                let itself = self.untagged_changed_itself(ty);\n",
    tests: ["test/corpus.test.ts", "-t", "untagged_enums"],
  },
  {
    name: "derived-clone-shares-changed-vec",
    breaks:
      "a derived `clone` is `{ ...s }`, sharing the `Vec` the copy is pushed to, so the push shows in the original too",
    file: "src/lower/std_impls/clones.rs",
    find: "                    .map_or(self.needs_clone(t), |item| self.needs_clone(item))\n            })\n",
    replace: "                    .map_or(self.needs_clone(t), |item| self.needs_clone(item))\n                    && false\n            })\n",
    tests: [
      "test/compiler.test.ts",
      "-t",
      "std trait impls are direct calls, and a clone copies only what changes",
    ],
  },
  {
    name: "map-clone-shares-values",
    breaks:
      "a clone of a `HashMap` of `Vec`s is `new Map(m)`, sharing each `Vec`, so a push to the clone's shows in the original's",
    file: "src/lower/std_impls/clones.rs",
    find: "        let value = value_ty.filter(|&v| self.needs_clone(v));",
    replace: "        let value = value_ty.filter(|&v| false && self.needs_clone(v));",
    tests: [
      "test/compiler.test.ts",
      "-t",
      "HashMap and HashSet are a JS Map and Set",
    ],
  },
  {
    name: "clone-cell-field-respread",
    breaks:
      "a derived clone of a `Cell` or a `OnceCell` field copies it again, `{ ...c, count: c.count }`, which the spread did",
    file: "src/lower/std_impls/clones.rs",
    find: "                self.property_cell(def(self, i), t)\n                    .map_or(self.needs_clone(t), |item| self.needs_clone(item))\n",
    replace: "                self.needs_clone(t)\n",
    tests: [
      "test/lowering-cells.test.ts",
      "-t",
      "cell in a field is the property",
    ],
  },
  {
    name: "enum-clone-whole-shared",
    breaks: "a clone of an enum changed in place is the same object",
    file: "src/lower/std_impls/clones.rs",
    find: "                    return self.clone_fields(place, (ty, None), fields[0].clone(), span, out);\n",
    replace: "                    return Ok(place);\n",
    tests: ["test/corpus.test.ts", "-t", "or_bindings"],
  },
  {
    name: "refcell-clone-unheld",
    breaks:
      "a `RefCell`'s clone of what may ask runs with it free to borrow mutably",
    file: "src/lower/std_impls/clones.rs",
    find: "            ty::Adt(_, args) if std(StdItem::RefCell) && !self.recognition().std_alone(args.type_at(0)) => {\n                // Its parameter named apart",
    replace:
      "            ty::Adt(_, args) if false => {\n                // Its parameter named apart",
    tests: ["test/corpus.test.ts", "-t", "^refcell_clone_nested_parts.rs$"],
    snapshots: true,
  },
  {
    name: "refcell-clone-unchecked",
    breaks: "a `RefCell`'s clone while it's mutably borrowed runs on",
    file: "src/lower/std_impls/clones.rs",
    find: "l = match std(StdItem::RefCell)",
    replace: "l = match false",
    tests: ["test/corpus.test.ts", "-t", "refcell_clone_number_while_mut"],
  },
  {
    name: "mir-array-iterator-clone-shared",
    breaks:
      "a clone of std's iterator over an array from MIR is the same `$iter`, stepped by both",
    file: "src/lower/std_impls/clones.rs",
    find: "            ty::Adt(..) if self.steps_an_array(ty) => {\n",
    replace:
      "            ty::Adt(..) if false && self.steps_an_array(ty) => {\n",
    tests: ["test/corpus.test.ts", "-t", "iterator_clones"],
  },
  {
    name: "std-hasher-clone-shared",
    breaks: "a cloned std hasher is the same one",
    file: "src/lower/std_impls/clones.rs",
    find: '                Ok(Expr::call(Expr::member(place, "clone"), Vec::new()))\n',
    replace: "                Ok(place)\n",
    tests: ["test/corpus.test.ts", "-t", "^hash_values.rs$"],
  },
  {
    name: "cloned-apart-is-itself",
    breaks: "a clone of a closure cloned apart is the closure itself",
    file: "src/lower/std_impls/clones.rs",
    find: '                Some(false) => return Ok(Expr::call_helper("$clone", vec![place])),\n',
    replace: "                Some(false) => return Ok(place),\n",
    tests: ["test/corpus.test.ts", "-t", "^closure_clone.rs$"],
  },
  {
    name: "cloned-apart-field-shared",
    breaks: "a struct's clone shares a field's closure cloned apart",
    file: "src/lower/std_impls/clones.rs",
    find: "            ty::Closure(id, _) => self.krate.closure_facts.clones_apart(*id),\n",
    replace:
      "            ty::Closure(id, _) => false && self.krate.closure_facts.clones_apart(*id),\n",
    tests: ["test/corpus.test.ts", "-t", "^closure_clone_counted.rs$"],
  },
  {
    name: "iterator-clone-is-itself",
    breaks: "a clone of a `map` cloned apart is the `map` itself",
    file: "src/lower/std_impls/clones.rs",
    find: '        if (self.iterator_cloned_apart(ty) && !held_compiled) || self.recognition().is_memchr_iter(ty) {\n            return Ok(Expr::call_helper("$clone", vec![place]));\n',
    replace:
      "        if (self.iterator_cloned_apart(ty) && !held_compiled) || self.recognition().is_memchr_iter(ty) {\n            return Ok(place);\n",
    tests: ["test/corpus.test.ts", "-t", "^lazy_iterator_clone.rs$"],
  },
  {
    name: "copied-clone-refused",
    breaks:
      "a clone of a `copied()` is refused, though it's the iterator it copies",
    file: "src/lower/std_impls/clones.rs",
    find: "            && !held_compiled\n        {\n            return self.copied_clone(place, (ty, source), span, out);\n",
    replace:
      "            && false\n        {\n            return self.copied_clone(place, (ty, source), span, out);\n",
    tests: [
      "test/lowering-iterators.test.ts",
      "-t",
      "a clone of a generic iterator.s copied",
    ],
  },
  {
    name: "memchr-iter-clone-refused",
    breaks:
      "a clone of memchr's iterator is refused, though it carries its clone",
    file: "src/lower/std_impls/clones.rs",
    find: "        if (self.iterator_cloned_apart(ty) && !held_compiled) || self.recognition().is_memchr_iter(ty) {\n",
    replace: "        if self.iterator_cloned_apart(ty) && !held_compiled {\n",
    tests: ["test/shipped-crates.test.ts", "-t", "memchr's searches run"],
  },
  {
    name: "iter-clone-items-shared",
    breaks:
      "a clone of a `Vec`'s `into_iter()` shares its items, which change in place",
    file: "src/lower/std_impls/clones.rs",
    find: "                let owns = self.array_source(ty).unwrap_or(false);\n",
    replace: "                let owns = false;\n",
    tests: ["test/corpus.test.ts", "-t", "^std_iterator_clones.rs$"],
  },
  {
    name: "slice-clone-refused",
    breaks: "a clone of a `Box<[T]>`'s slice is refused",
    file: "src/lower/std_impls/clones.rs",
    find: "            ty::Array(item, _) | ty::Slice(item) => return self.clone_items(place, *item, span),\n",
    replace:
      "            ty::Array(item, _) => return self.clone_items(place, *item, span),\n",
    tests: ["test/corpus.test.ts", "-t", "^boxed_slice_values.rs$"],
  },
  {
    name: "slice-needs-no-clone",
    breaks: "a clone of a `Box<[T]>` shares its array",
    file: "src/lower/std_impls/clones.rs",
    find: "            ty::Slice(item) => self.items_changed(*item) || self.needs_clone_in(*item, seen),\n",
    replace: "            ty::Slice(_) => false,\n",
    tests: ["test/corpus.test.ts", "-t", "^boxed_slice_values.rs$"],
  },
  {
    name: "manually-drop-clone-refused",
    breaks: "a `ManuallyDrop`'s clone is refused, where it's its value's",
    file: "src/lower/std_impls/clones.rs",
    find: "            ty::Adt(..) if let Some(held) = self.recognition().std_wrapped(ty) => {\n                return self.clone_value(place, held, span, out);\n",
    replace:
      '            ty::Adt(..) if false => {\n                return Err(self.unsupported(span, ""));\n',
    tests: ["test/corpus.test.ts", "-t", "^manually_drop_values.rs$"],
  },
  {
    name: "parse-error-clone-refused",
    breaks: "a ParseIntError's clone is cloned as itself",
    file: "src/lower/std_impls/clones.rs",
    find: "                    || self.is_parse_error(ty) =>",
    replace: "                    =>",
    tests: ["test/corpus.test.ts", "-t", "str_odds"],
  },
  {
    name: "refcell-clone-param-named-alone",
    breaks:
      "a `RefCell`'s clone's arrow names its parameter `value` itself, which a part its body spills is named too: `(value) => { const value = ..`, JS that doesn't parse (#200)",
    file: "src/lower/std_impls/clones.rs",
    find: '                // part read more than once of a `RefCell` inside (#200).\n                let param = self.fresh("value");\n',
    replace:
      '                // part read more than once of a `RefCell` inside (#200).\n                let param = String::from("value");\n',
    tests: ["test/corpus.test.ts", "-t", "^refcell_clone_nested_parts.rs$"],
  },
  {
    name: "value-cell-cloned-boxed",
    breaks:
      "a `RefCell` of an object's clone is a `{ value }` of the clone, read as the object it isn't",
    file: "src/lower/std_impls/clones.rs",
    find: '                    return Ok(Expr::call(\n                        Expr::helper("$withBorrow"),\n                        vec![place, Expr::arrow(vec![param.as_str().into()], body), Expr::bool(true)],\n                    ));\n',
    replace:
      '                    return Ok(Expr::object(vec![Prop::Field(js::HELD.into(), Expr::call(\n                        Expr::helper("$withBorrow"),\n                        vec![place, Expr::arrow(vec![param.as_str().into()], body), Expr::bool(true)],\n                    ))]));\n',
    tests: ["test/corpus.test.ts", "-t", "^refcell_objects.rs$"],
  },
  {
    name: "transparent-of-no-object-cloned",
    breaks:
      'a derived `Clone` of a transparent struct of a function, written over where it\'s kept, is refused, "cloning `Task`" (ADR 0584, amended)',
    file: "src/lower/std_impls/clones.rs",
    find: "            && !self.may_be_object(held)\n            && !self.has_user_impl(self.clone_trait(), ty)\n",
    replace: "            && false\n",
    tests: ["test/corpus.test.ts", "-t", "^refcell_clone_through_rc.rs$"],
  },
  {
    name: "held-map-cloned-apart",
    breaks:
      "a held `Map`, std's struct, is cloned as an adapter cloned apart, `$clone`, which it isn't",
    file: "src/lower/std_impls/clones.rs",
    find: "        if (self.iterator_cloned_apart(ty) && !held_compiled) || self.recognition().is_memchr_iter(ty) {\n",
    replace:
      "        if self.iterator_cloned_apart(ty) || self.recognition().is_memchr_iter(ty) {\n",
    tests: ["test/corpus.test.ts", "-t", "^(std_map|lazy_iterator_clone).rs$"],
  },
  {
    name: "range-inclusive-clone-unexhausted",
    breaks:
      "a clone of an exhausted inclusive range copies its bounds alone, not exhausted, `3..=3` where std's clone is `3..=3 (exhausted)` (#414)",
    file: "src/lower/std_impls/clones.rs",
    find: "                Ok(Expr::object(vec![Prop::Spread(place)]))\n",
    replace:
      '                Ok(Expr::object(vec![Prop::Field("start".into(), Expr::member(place.clone(), "start")), Prop::Field("end".into(), Expr::member(place, "end"))]))\n',
    tests: ["test/corpus.test.ts", "-t", "^range_inclusive_exhausted.rs$"],
  },
  {
    name: "untagged-arrays-clone-shared",
    breaks:
      "a clone of an untagged enum of arrays changed in place is the array itself, where its payload needs no clone of its own, which `*place = ..` then changes too (rd490)",
    file: "src/lower/std_impls/clones.rs",
    find: "                let itself = self.untagged_changed_itself(ty);\n",
    replace: "                let itself = false;\n",
    tests: ["test/corpus.test.ts", "-t", "untagged_told_by_length"],
  },
  {
    name: "variant-clone-boxes-cells",
    breaks: "a derived `Clone` of an enum reads its variant's `RefCell` field that holds its value as `{ value }`, `{ ...a, runs: { value: $borrow(a.runs).value } }` (#503)",
    file: "src/lower/std_impls/clones.rs",
    find: "            Some(variant) => variant.fields.get(rustc_abi::FieldIdx::from_usize(i)),\n",
    replace: "            Some(_) => None,\n",
    tests: ["test/corpus.test.ts", "-t", "^refcell_variant_derives.rs$"],
  },
  {
    name: "variant-clone-of-cells-shared",
    breaks: "a derived `Clone` of an enum whose variant holds a cell that's the property it holds is the value it clones, `const b = a`, so a write to one's cell is the other's (#503)",
    file: "src/lower/std_impls/clones.rs",
    find: "                    (variant.fields.iter().zip(fields)).any(|(def, &(_, t))| cx.property_cell(Some(def), t).is_some())\n",
    replace: "                    (variant.fields.iter().zip(fields)).any(|(def, &(_, t))| cx.property_cell(Some(def), t).is_some() && false)\n",
    tests: ["test/corpus.test.ts", "-t", "^refcell_variant_derives.rs$"],
  },
  {
    name: "cloned-option-held-first",
    breaks:
      "an `Option<&T>`'s `cloned()` of a type parameter's, whose clone is itself, is held in a `const` first, `const value = items.at(-1); return value;` (#587)",
    file: "src/lower/std_impls/clones.rs",
    find: "                        self.names.swap_remove(held);\n                        out.pop();\n                        return Ok(written);\n",
    replace: "",
    tests: ["test/corpus.test.ts", "-t", "^cloned_option_of_a_type_parameter.rs$"],
    snapshots: true,
  },
  {
    name: "clone-of-nodes-unshaped",
    breaks: "a clone of a tree that may be std's nodes isn't given its source, so it's made in another shape (#926)",
    file: "src/lower/std_impls/clones.rs",
    find: "                let shape = nodes.then(|| place.clone());\n",
    replace: "                let shape = None::<Expr>;\n",
    tests: ["test/corpus.test.ts", "-t", "^btree_clone_nodes_unwinds.rs$|^btree_nodes_seen.rs$"],
  },
  {
    name: "chars-clone-refused",
    breaks: "a clone of a `chars()` or `char_indices()`, `$chars`' own `clone()`, is refused",
    file: "src/lower/std_impls/clones.rs",
    find: "                    || self.recognition().is_str_chars(ty)\n                    || self.recognition().is_char_indices(ty) =>\n",
    replace: "                    || false =>\n",
    tests: ["test/corpus.test.ts", "-t", "^std_iterator_clones.rs$"],
  },
  {
    name: "closure-clone-itself-reasked",
    breaks:
      "a clone of a closure the crate decided is itself is asked again of what the crate can see of it, so a library's `map(|s| s.len())`, which rustc infers `FnMut`, is refused (#1202)",
    file: "src/lower/std_impls/clones.rs",
    find: "                Some(true) => return Ok(place),\n",
    replace: "                Some(true) => {}\n",
    tests: ["test/crates.test.ts", "-t", "two crates: library_closures_cloned_as_made"],
  },
  {
    name: "closure-clone-apart-unmade",
    breaks: "a clone of a closure cloned apart is asked again, so a library's `counter()`, which writes what it took, is refused, or is the closure itself (ADR 0522)",
    file: "src/lower/std_impls/clones.rs",
    find: "                Some(false) => return Ok(Expr::call_helper(\"$clone\", vec![place])),\n",
    replace: "                Some(false) => {}\n",
    tests: ["test/corpus.test.ts", "-t", "^closure_clone.rs$"],
  },
  {
    name: "iter-clone-loses-lowerless",
    breaks: "a clone of a B-tree's `range(..)` is `$iter` of what's left with no mark, whose hint counts it, so a crate that reads a capacity grows a collect of it otherwise (#1215)",
    file: "src/lower/std_impls/clones.rs",
    find: "                Ok(self.stepped_items_of(left, ty))\n",
    replace: "                Ok(self.stepped_items(left))\n",
    tests: ["test/iterator-forms.test.ts", "-t", "capacity_of_what_std_hints_less"],
  },
  {
    name: "clone-fn-parameter-free-to-its-body",
    breaks:
      "a `Clone` dictionary's `clone(value)` of an `Option<Option<T>>` binds what its `Some` holds as `const value`, its own parameter's name again, so the module doesn't parse (#1246)",
    file: "src/lower/std_impls/clones.rs",
    find: "        let taken = self.names.insert(name.to_string());\n",
    replace: "        let taken = false;\n",
    tests: ["test/corpus.test.ts", "-t", "^clone_dictionary_of_a_nested_option.rs$"],
  },
  {
    name: "map-entries-unguarded",
    breaks: "a clone of a map whose value's clone panics leaves the entries it made undropped (#897)",
    file: "src/lower/std_impls/clones.rs",
    find: "                let made = match may_panic && self.drops(entry) != crate::lower::drops::Drops::Nothing {\n",
    replace: "                let made = match false && self.drops(entry) != crate::lower::drops::Drops::Nothing {\n",
    tests: ["test/corpus.test.ts", "-t", "^clone_unwinds_what_it_made.rs$"],
  },
  {
    name: "compiled-copied-cloned-as-source",
    breaks:
      "a `copied()` std compiles held, `--std-iterators`', is cloned as the crate's iterator it copies, `held.it.with(..)`: `TypeError: held.it.with is not a function` (ADR 0999)",
    file: "src/lower/std_impls/clones.rs",
    find: "        if let Some(source) = crate::lower::recognition::copied_source(self.tcx, ty)\n            && !held_compiled\n",
    replace:
      "        if let Some(source) = crate::lower::recognition::copied_source(self.tcx, ty)\n",
    tests: ["test/iterator-forms.test.ts", "-t", "copied_clone_of_its_own"],
  },
  {
    name: "va-list-clone-unknown",
    breaks:
      "a clone of a `VaList` is an error, where it's a copy of what's left",
    file: "src/lower/std_impls/clones.rs",
    find: "            ty::Adt(..) if crate::lower::recognition::is_va_list(self.tcx, ty) => {\n",
    replace: "            ty::Adt(..) if false => {\n",
    tests: ["test/corpus.test.ts", "-t", "^c_variadic.rs$"],
  },
  {
    name: "copies-of-shared",
    breaks:
      "what std copies of `Copy` structs, `extend` of references, `concat`, `partition`, is the same objects, so changing a copy changes the original",
    file: "src/lower/std_impls/clones.rs",
    find: "    pub(in crate::lower) fn copies_of(&mut self, items: Expr, ty: Ty<'tcx>, span: Span) -> R<Expr> {\n        if !self.needs_clone(ty) {",
    replace:
      "    pub(in crate::lower) fn copies_of(&mut self, items: Expr, ty: Ty<'tcx>, span: Span) -> R<Expr> {\n        if true || !self.needs_clone(ty) {",
    tests: ["test/corpus.test.ts", "-t", "^copied_structs_apart.rs$"],
  },
  {
    name: "held-cycle-clone-refused",
    breaks: "a held `Cycle`'s clone is refused",
    file: "src/lower/std_impls/clones.rs",
    find: "                if self.explicit_adapter(ty) && crate::lower::recognition::cycled_source(self.tcx, ty).is_some() =>\n",
    replace:
      "                if self.explicit_adapter(ty) && crate::lower::recognition::cycled_source(self.tcx, ty).is_some() && false =>\n",
    tests: ["test/corpus.test.ts", "-t", "^held_cycle.rs$"],
  },
  {
    name: "boxed-slice-items-changed-unasked",
    breaks:
      "a boxed slice's clone, a `CString`'s too, is the same array where what `into_vec()` or `into_bytes_with_nul()` makes of it is changed in place (#1290)",
    file: "src/lower/std_impls/clones.rs",
    find: "            ty::Slice(item) => self.items_changed(*item) || self.needs_clone_in(*item, seen),\n",
    replace: "            ty::Slice(item) => self.needs_clone_in(*item, seen),\n",
    tests: ["test/corpus.test.ts", "-t", "^clone_changed_as_a_vec_or_a_boxed_slice.rs$"],
  },
  {
    name: "vec-items-changed-unasked",
    breaks:
      "a `Vec`'s clone is the same array where the boxed slice its `into_boxed_slice()` makes is changed in place (#1290)",
    file: "src/lower/std_impls/clones.rs",
    find: "                    || (std(StdItem::Vec) && self.items_changed(args.type_at(0)))\n",
    replace: "",
    tests: ["test/corpus.test.ts", "-t", "^clone_changed_as_a_vec_or_a_boxed_slice.rs$"],
  },
];
