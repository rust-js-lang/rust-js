// Mutations of src/lower/std_types/map_keys.rs (ADR 0093).
import type { Mutation } from "../../../mutations";

export const mutations: Mutation[] = [
  {
    name: "option-keys-by-identity",
    breaks:
      "a `HashMap<Option<u32>, _>` is a plain `Map`, whose `null` key and `undefined` key are two, where Rust has one `None`, and `get(&None)` misses a nullable field's (ADR 0935)",
    file: "src/lower/std_types/map_keys.rs",
    find: "        self.is_primitive_key(ty.peel_refs())\n",
    replace:
      "        let ty = ty.peel_refs();\n        self.is_primitive_key(ty) || self.option_of(ty).is_some_and(|inner| self.is_primitive_key(inner.peel_refs()))\n",
    tests: ["test/corpus.test.ts", "-t", "^null_option_keys.rs$"],
  },
  {
    name: "ordered-key-custom-ord",
    breaks:
      "a `BTreeMap` of a key with its own `Ord` is kept in the key's JS order, not the order its `cmp` says",
    file: "src/lower/std_types/map_keys.rs",
    find: "            && (!ordered || !self.has_user_impl(self.ord_trait(), peeled));",
    replace: ";",
    tests: ["test/semantics.test.ts", "-t", "finds enum keys by their own Ord"],
  },
  {
    name: "path-key-refused",
    breaks: "a map keyed by paths is refused",
    file: "src/lower/std_types/map_keys.rs",
    find: "            || self.recognition().is_path(peeled)\n            || self.is_cow_key(peeled)\n",
    replace: "            || self.is_cow_key(peeled)\n",
    tests: ["test/corpus.test.ts", "-t", "^(path_keys|cow_keys).rs$"],
  },
  {
    name: "cow-key-slice-refused",
    breaks: "a map of `Cow<[T]>`s is refused",
    file: "src/lower/std_types/map_keys.rs",
    find: "            ty::Slice(item) => self.compares_by_value(*item, &mut Vec::new()) && !self.holds_identity(*item),\n",
    replace: "            ty::Slice(_) => false,\n",
    tests: ["test/corpus.test.ts", "-t", "^cow_keys.rs$"],
  },
  {
    name: "cow-key-given-back",
    breaks:
      "a `Cow` map's key is given back as it was asked with, a `&str` where it holds a `Cow`",
    file: "src/lower/std_types/map_keys.rs",
    find: "        self.is_cow_key(key)\n            || self.recognition().is_path(key)\n",
    replace: "        self.recognition().is_path(key)\n",
    tests: ["test/corpus.test.ts", "-t", "^held_keys.rs$"],
  },
  {
    name: "cow-key-refused",
    breaks: "a map keyed by `Cow`s is refused",
    file: "src/lower/std_types/map_keys.rs",
    find: "            || self.recognition().is_path(peeled)\n            || self.is_cow_key(peeled)\n",
    replace: "            || self.recognition().is_path(peeled)\n",
    tests: ["test/corpus.test.ts", "-t", "^cow_keys.rs$"],
  },
  {
    name: "own-ord-tree-made-as-inserted",
    breaks:
      "a B-tree of its own `Ord` made of items keeps the first of equal ones, as `insert` does",
    file: "src/lower/std_types/map_keys.rs",
    find: "                true => self.found_by_cmp(key),\n",
    replace: "                true => false,\n",
    tests: ["test/corpus.test.ts", "-t", "^btree_own_ord.rs$"],
  },
  {
    name: "value-key-trees-refused",
    breaks:
      "a B-tree keyed by a tuple, or by a struct whose `Ord` is derived, is an ordered tree of its `cmp`, `$ordMap`, not a `$KeyMap` of its value",
    file: "src/lower/std_types/map_keys.rs",
    find: "            || (self.is_value_key(ty) && (!ordered || self.orders_by_value(ty, &mut Vec::new())))\n",
    replace: "            || (self.is_value_key(ty) && !ordered)\n",
    tests: ["test/corpus.test.ts", "-t", "^value_key_trees.rs$"],
    snapshots: true,
  },
  {
    name: "value-key-tree-own-ord",
    breaks:
      "a B-tree keyed by a struct whose `Ord` is the crate's own is a `$KeyMap`, which finds a key by its fields",
    file: "src/lower/std_types/map_keys.rs",
    find: "            ty::Adt(adt, args) => {\n                self.recognition().derives(self.ord_trait(), ty)\n                    && adt\n",
    replace: "            ty::Adt(adt, args) => {\n                adt\n",
    tests: ["test/corpus.test.ts", "-t", "^btree_own_ord.rs$"],
    snapshots: true,
  },
  {
    name: "value-key-tree-own-enum-ord",
    breaks:
      "a B-tree keyed by a tuple holding a fieldless enum whose `Ord` is the crate's own is a `$KeyMap`, which finds a key by its fields",
    file: "src/lower/std_types/map_keys.rs",
    find: "                !self.krate.foreign.compiled_by_rust_js(adt.did()) || self.recognition().derives(self.ord_trait(), ty)\n",
    replace: "                true || !self.krate.foreign.compiled_by_rust_js(adt.did())\n",
    tests: ["test/corpus.test.ts", "-t", "^btree_own_ord.rs$"],
    snapshots: true,
  },
  {
    name: "std-enum-key-refused",
    breaks: "a set of std's `Component`s, compared by value, is refused",
    file: "src/lower/std_types/map_keys.rs",
    find: "                    || self.std_enum_derives_eq(ty)\n",
    replace: "",
    tests: ["test/corpus.test.ts", "-t", "^path_components.rs$"],
  },
  {
    name: "identity-key-hashed",
    breaks: "a type equal only to itself is a `$hashedSet`'s key, hashing its address and asking its `eq`, not a JS `Set`'s of the objects (ADRs 0864, 1120)",
    file: "src/lower/std_types/map_keys.rs",
    find: "            && (!self.has_user_impl(self.partial_eq_trait(), peeled) || self.equal_only_to_itself(peeled))\n",
    replace:
      "            && !self.has_user_impl(self.partial_eq_trait(), peeled)\n",
    tests: ["test/corpus.test.ts", "-t", "^identity_keys.rs$"],
    snapshots: true,
  },
  {
    name: "identity-in-key-fields-unseen",
    breaks:
      "a struct's or an enum's field holding a fiber isn't seen, so the key is a `$KeyMap` of its fields (#398)",
    file: "src/lower/std_types/map_keys.rs",
    find: "                .any(|field| self.holds_identity_in(self.field_ty(field, args), seen)),\n",
    replace: "                .any(|_| false),\n",
    tests: ["test/corpus.test.ts", "-t", "^identity_inside_keys.rs$"],
  },
  {
    name: "identity-in-key-option-by-contents",
    breaks:
      "a `HashSet<Option<Rc<Fiber>>>` keys each `Some` by the fiber's fields (#398)",
    file: "src/lower/std_types/map_keys.rs",
    find: "                if self.js_eq(inner.peel_refs()).is_some() {\n                    return self.key_parts(key, inner, span);\n",
    replace:
      "                if self.js_eq(inner.peel_refs()).is_some() {\n                    return Ok(key);\n",
    tests: ["test/corpus.test.ts", "-t", "^identity_inside_keys.rs$"],
  },
  {
    name: "identity-in-key-option-none-read",
    breaks:
      "an `Option` of a struct holding a fiber reads the `None`'s fields, a throw (#398)",
    file: "src/lower/std_types/map_keys.rs",
    find: "                Ok(Expr::cond(Expr::presence(key.clone(), false), key, some))\n",
    replace: "                Ok(some)\n",
    tests: ["test/corpus.test.ts", "-t", "^identity_inside_keys.rs$"],
  },
  {
    name: "identity-in-key-variant-skipped",
    breaks:
      "an enum's variant holding a fiber keys it by the fiber's fields (#398)",
    file: "src/lower/std_types/map_keys.rs",
    find: "                    if !fields.iter().any(|&(_, t)| self.holds_identity(t)) {\n                        continue;\n",
    replace:
      "                    if !fields.iter().any(|&(_, t)| self.holds_identity(t)) || index.as_u32() == 0 {\n                        continue;\n",
    tests: ["test/corpus.test.ts", "-t", "^identity_inside_keys.rs$"],
  },
  {
    name: "identity-in-key-items-unmapped",
    breaks: "a `Vec` of fibers as a key is keyed by each fiber's fields (#398)",
    file: "src/lower/std_types/map_keys.rs",
    find: '        let each = self.key_parts_fn("item", item, span)?;\n        Ok(Expr::call(Expr::member(items, "map"), vec![each]))\n',
    replace: "        let _ = (item, span);\n        Ok(items)\n",
    tests: ["test/corpus.test.ts", "-t", "^identity_inside_keys.rs$"],
  },
  {
    name: "identity-in-key-tuple-part-kept",
    breaks:
      "a tuple key `(Rc<Fiber>, u32)` keys its fiber by its fields (#398)",
    file: "src/lower/std_types/map_keys.rs",
    find: "                    .map(|(i, t)| self.key_parts(Expr::index(key.clone(), Expr::int(i as i128)), t, span))\n",
    replace:
      "                    .map(|(i, _)| Ok(Expr::index(key.clone(), Expr::int(i as i128))))\n",
    tests: ["test/corpus.test.ts", "-t", "^identity_inside_keys.rs$"],
  },
  {
    name: "identity-in-key-nested-struct-kept",
    breaks:
      "a struct holding a struct holding a fiber keys the inner one by the fiber's fields (#398)",
    file: "src/lower/std_types/map_keys.rs",
    find: "                            let part = self.key_parts(self.project(key.clone(), ty, i), t, span)?;\n",
    replace:
      "                            let part = match self.js_eq(t.peel_refs()) {\n                                Some(_) => self.key_parts(self.project(key.clone(), ty, i), t, span)?,\n                                None => self.project(key.clone(), ty, i),\n                            };\n",
    tests: ["test/corpus.test.ts", "-t", "^identity_inside_keys.rs$"],
  },
  {
    name: "identity-in-cow-key-by-contents",
    breaks:
      "a `HashSet<Cow<[&Fiber]>>` is a `$CowSet`, which keys each fiber by its fields (#398)",
    file: "src/lower/std_types/map_keys.rs",
    find: "                ty::Slice(item) => self.compares_by_value(*item, &mut Vec::new()) && !self.holds_identity(*item),\n",
    replace:
      "                ty::Slice(item) => self.compares_by_value(*item, &mut Vec::new()),\n",
    tests: [
      "test/lowering-collections.test.ts",
      "-t",
      "a type equal only to itself keys a JS Set by itself",
    ],
  },
  {
    name: "phantom-data-key-not-by-value",
    breaks: "a key holding a `PhantomData` is a `$hashedMap`'s, hashing it, not a `$KeyMap`'s by its value (#514, ADR 1120)",
    file: "src/lower/std_types/map_keys.rs",
    find: "            // a field left out of its object, is to `$key` (#514).\n            ty::Adt(..) if self.is_lang_adt(ty, LangItem::PhantomData) => true,\n",
    replace: "",
    tests: ["test/corpus.test.ts", "-t", "^hash_phantom_data.rs$"],
    snapshots: true,
  },
  {
    name: "phantom-data-key-not-ordered-by-value",
    breaks:
      "a B-tree keyed by a struct holding a `PhantomData`, litemap's `LiteMap`, finds it by its `cmp`, an `$OrdMap`, not by its value, a `$KeySet` (#514)",
    file: "src/lower/std_types/map_keys.rs",
    find: "            // Every `PhantomData` is `Equal` to every other (#514).\n            ty::Adt(..) if self.is_lang_adt(ty, LangItem::PhantomData) => true,\n",
    replace: "",
    tests: ["test/corpus.test.ts", "-t", "^hash_phantom_data.rs$"],
    snapshots: true,
  },
  {
    name: "c-string-key-not-by-value",
    breaks: "a map of `CString` keys is refused",
    file: "src/lower/std_types/map_keys.rs",
    find: "                    || crate::lower::recognition::is_c_string_like(self.tcx, ty))",
    replace: ")",
    tests: ["test/corpus.test.ts", "-t", "^c_strings.rs$"],
  },
  {
    name: "hashed-key-own-hash-unseen",
    breaks: "a key whose `==` is derived but whose `hash` is its own is a `$KeyMap`, which never calls it: a `hash` that panics, std's table calls, never runs (ADR 1120)",
    file: "src/lower/std_types/map_keys.rs",
    find: "        (!self.is_key(key, false) && !key.peel_refs().is_unit()) || self.hash_runs_code(key, &mut Vec::new())\n",
    replace: "        !self.is_key(key, false) && !key.peel_refs().is_unit()\n",
    tests: ["test/corpus.test.ts", "-t", "^keys_found_by_their_own_hash.rs$"],
  },
  {
    name: "hashed-key-given-back",
    breaks: "a hashed set's `get` and `take` of a key found by its own `Eq` give back the key asked with, not the one the set holds (ADR 1120)",
    file: "src/lower/std_types/map_keys.rs",
    find: "                false => self.found_by_hash(key),\n",
    replace: "                false => false,\n",
    tests: ["test/corpus.test.ts", "-t", "^keys_found_by_their_own_hash.rs$"],
  },
];
