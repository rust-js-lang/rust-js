// Mutations of src/lower/std_types/hash.rs (ADR 0463).
import type { Mutation } from "../../../mutations";

export const mutations: Mutation[] = [
  {
    name: "hash-one-of-nothing",
    breaks: "`hash_one(x)` of a `BuildHasherDefault<DefaultHasher>` writes nothing of `x`, every value's hash an empty hasher's (#1155)",
    file: "src/lower/std_types/hash.rs",
    find: "                self.hash_writes(value, call.generic_args.type_at(1), &hasher, call.span, out)?;\n",
    replace: "                let _ = (value, &hasher);\n",
    tests: ["test/corpus.test.ts", "-t", "^build_hasher_default.rs$"],
  },
  {
    name: "hash-default-hasher-rounds",
    breaks: "`DefaultHasher` is SipHash-2-4, `SipHasher`'s, not SipHash-1-3",
    file: "src/lower/std_types/hash.rs",
    find: "        StdHasher::Default => [1, 3],\n",
    replace: "        StdHasher::Default => [2, 4],\n",
    tests: ["test/corpus.test.ts", "-t", "^hash_values.rs$"],
  },
  {
    name: "hash-keys-dropped",
    breaks: "`SipHasher::new_with_keys(k0, k1)` hashes as zero keys",
    file: "src/lower/std_types/hash.rs",
    find: "            HashOp::NewWithKeys => new_hasher(StdHasher::Sip, values.collect()),\n",
    replace:
      "            HashOp::NewWithKeys => new_hasher(StdHasher::Sip, Vec::new()),\n",
    tests: ["test/corpus.test.ts", "-t", "^hash_hashers.rs$"],
  },
  {
    name: "hash-std-hasher-by-dictionary",
    breaks:
      "a std hasher's writes are its dictionary's, `$sipHasher.write_u32({ value: h }, n)`, not `h.write_u32(n)`",
    file: "src/lower/std_types/hash.rs",
    find: "        let own = self.recognition().std_hasher(ty).is_some();\n",
    replace: "        let own = false;\n",
    tests: ["test/corpus.test.ts", "-t", "^hash_values.rs$"],
    snapshots: true,
  },
  {
    name: "hash-crate-hasher-unboxed",
    breaks:
      "a crate's hasher is given to its dictionary as the object, where its methods read a box's `value`",
    file: "src/lower/std_types/hash.rs",
    find: "        let held = match !own && self.mut_ref_form(ty).is_cell() {\n",
    replace: "        let held = match true {\n",
    tests: ["test/corpus.test.ts", "-t", "^hash_hashers.rs$"],
  },
  {
    name: "hash-char-as-text",
    breaks:
      "a `char` is hashed as text, `write_str`, not as its code point, a `u32`",
    file: "src/lower/std_types/hash.rs",
    find: '                out.push(hasher.write(&self.hasher_key("write_u32"), vec![code]));\n',
    replace:
      '                out.push(hasher.write(&self.hasher_key("write_str"), vec![code]));\n',
    tests: ["test/corpus.test.ts", "-t", "^hash_values.rs$"],
  },
  {
    name: "hash-slice-unprefixed",
    breaks:
      "a slice's length isn't written before its items, so `([1], [2])` and `([1, 2], [])` hash alike",
    file: "src/lower/std_types/hash.rs",
    find: '                out.push(hasher.write(\n                    &self.hasher_key("write_length_prefix"),\n                    vec![Expr::member(value.clone(), "length")],\n                ));\n',
    replace: "",
    tests: ["test/corpus.test.ts", "-t", "^hash_values.rs$"],
  },
  {
    name: "hash-option-discriminants-swapped",
    breaks: "`Some` is hashed as discriminant 0 and `None` as 1",
    file: "src/lower/std_types/hash.rs",
    find: '                let mut some = vec![hasher.write(&self.hasher_key("write_isize"), vec![Expr::int(1)])];\n',
    replace:
      '                let mut some = vec![hasher.write(&self.hasher_key("write_isize"), vec![Expr::int(0)])];\n',
    tests: ["test/corpus.test.ts", "-t", "^hash_values.rs$"],
  },
  {
    name: "hash-result-arms-swapped",
    breaks: "an `Ok` is hashed as `Err`'s discriminant",
    file: "src/lower/std_types/hash.rs",
    find: "                for (discriminant, held) in [(0, ok), (1, err)] {\n",
    replace:
      "                for (discriminant, held) in [(1, ok), (0, err)] {\n",
    tests: ["test/corpus.test.ts", "-t", "^hash_values.rs$"],
  },
  {
    name: "hash-own-hash-slice-ignored",
    breaks:
      "a slice of a crate's type is hashed item by item, not by its own `hash_slice`",
    file: "src/lower/std_types/hash.rs",
    find: '            return self.dictionary_writes("hash_slice", items, item, hasher, span, out);\n',
    replace:
      "            return self.each_item_writes(items, item, hasher, span, out);\n",
    tests: ["test/corpus.test.ts", "-t", "^hash_values.rs$"],
  },
  {
    name: "hash-integer-slice-each",
    breaks:
      "an integer slice's items are each a write, where std's are one `write` of their bytes",
    file: "src/lower/std_types/hash.rs",
    find: "        let Some(num) = (matches!(item.kind(), ty::Int(_) | ty::Uint(_)))\n",
    replace: "        let Some(num) = (false)\n",
    tests: ["test/corpus.test.ts", "-t", "^hash_hashers.rs$"],
  },
  {
    name: "hash-bytes-mapped",
    breaks:
      "a `[u8]`'s bytes are each mapped to themselves, `bytes.flatMap(..)`, not written as they are",
    file: "src/lower/std_types/hash.rs",
    find: "            8 if !num.signed() => items,\n",
    replace: "",
    tests: ["test/corpus.test.ts", "-t", "^hash_values.rs$"],
    snapshots: true,
  },
  {
    name: "ptr-hash-of-pointee",
    breaks: "`ptr::hash(p, state)` hashes what `p` points at, not its address",
    file: "src/lower/std_types/hash.rs",
    find: "                        let pointer = ty::Ty::new_imm_ptr(self.tcx, hashed);\n",
    replace: "                        let pointer = hashed;\n",
    tests: ["test/corpus.test.ts", "-t", "^identity_keys.rs$"],
  },
  {
    name: "nothing-written-looped",
    breaks: "a slice of `PhantomData`s' `hash_slice` loops over its items, `for (const item of items) {}`, writing nothing",
    file: "src/lower/std_types/hash.rs",
    find: "        if body.is_empty() {\n            return Ok(());\n        }\n",
    replace: "",
    tests: ["test/corpus.test.ts", "-t", "^hash_phantom_data.rs$"],
    snapshots: true,
  },
  {
    name: "hash-runs-code-of-derived-unwalked",
    breaks: "a derived `Hash` is taken to run no code of its fields, so a key holding one whose own `hash` panics never calls it (ADR 1120)",
    file: "src/lower/std_types/hash.rs",
    find: "                .any(|field| self.hash_runs_code(self.field_ty(field, args), seen)),\n",
    replace: "                .any(|_| false),\n",
    tests: ["test/corpus.test.ts", "-t", "^keys_found_by_their_own_hash.rs$"],
  },
  {
    name: "range-hash-exhausted-unwritten",
    breaks: "a hashed `a..=b` writes its bounds alone, never whether it's exhausted, so one stepped to its end hashes as a fresh one of its bounds, where std's derive writes its `exhausted` (#1361)",
    file: "src/lower/std_types/hash.rs",
    find: "                if kind == crate::lower::std_types::range::RangeKind::Inclusive {\n",
    replace: "                if false {\n",
    tests: ["test/corpus.test.ts", "-t", "^ranges_hashed_as_std_derives.rs$"],
  },
  {
    name: "range-hash-bounds-unwritten",
    breaks: "a hashed range writes none of its bounds, so every range of a kind hashes alike (#1361)",
    file: "src/lower/std_types/hash.rs",
    find: "                if let Some(bound) = bound {\n                    for each in bounds {\n",
    replace: "                if let Some(bound) = bound.filter(|_| false) {\n                    for each in bounds {\n",
    tests: ["test/corpus.test.ts", "-t", "^ranges_hashed_as_std_derives.rs$"],
  },

  {
    name: "std-enum-discriminant-unwritten",
    breaks: "a std enum's derived `Hash` writes no discriminant, so `Bound::Excluded(v)` hashes as `v` alone, not as `Some(v)` (#1289)",
    file: "src/lower/std_types/hash.rs",
    find: "                    if adt.variants().len() > 1 {\n",
    replace: "                    if false {\n",
    tests: ["test/corpus.test.ts", "-t", "^std_keys_around_their_own_hash.rs$"],
  },
  {
    name: "std-enum-discriminant-zero",
    breaks: "each variant of a std enum hashes discriminant 0, `Bound::Excluded(v)` not as `Some(v)` (#1289)",
    file: "src/lower/std_types/hash.rs",
    find: "                        arm.push(hasher.write(&write, vec![Expr::int(at)]));\n",
    replace: "                        arm.push(hasher.write(&write, vec![Expr::int(0 * at)]));\n",
    tests: ["test/corpus.test.ts", "-t", "^std_keys_around_their_own_hash.rs$"],
  },
  {
    name: "std-enum-hash-runs-no-code",
    breaks: "a std enum around a key whose `Hash` is its own is taken for one that runs none, a `$KeySet` that never calls it (#1289)",
    file: "src/lower/std_types/hash.rs",
    find: "                    (adt.all_fields()).any(|field| self.hash_runs_code(self.field_ty(field, args), seen))\n",
    replace: "                    (adt.all_fields()).any(|field| false && self.hash_runs_code(self.field_ty(field, args), seen))\n",
    tests: ["test/corpus.test.ts", "-t", "^std_keys_around_their_own_hash.rs$"],
  },
  {
    name: "hashed-made-array-parts-with-effects",
    breaks: "a tuple made of calls, hashed where it's made, has each part's call made as it's hashed, after an earlier part's `hash` ran, not all first as std makes the tuple (#1289)",
    file: "src/lower/std_types/hash.rs",
    find: "                    js::ExprKind::Array(items) if items.len() == parts.len() && !value.has_effects() => {\n",
    replace: "                    js::ExprKind::Array(items) if items.len() == parts.len() => {\n",
    tests: ["test/corpus.test.ts", "-t", "^std_keys_around_their_own_hash.rs$"],
  },
  {
    name: "hashed-made-array-held",
    breaks: "a `Wrapping(x)` hashed where it's made is held, `const value = [x]`, and read back, not `x` written as it is (#1289)",
    file: "src/lower/std_types/hash.rs",
    find: "                    js::ExprKind::Array(items) if items.len() == parts.len() && !value.has_effects() => {\n",
    replace: "                    js::ExprKind::Array(items) if false && items.len() == parts.len() && !value.has_effects() => {\n",
    tests: ["test/corpus.test.ts", "-t", "^hash_of_a_wrapping_in_generic_code.rs$"],
    snapshots: true,
  },
  {
    name: "layout-hash-align-first",
    breaks: "a `Layout` hashes its alignment before its size, not as std's derive (#1447)",
    file: "src/lower/std_types/hash.rs",
    find: "                for field in [\"size\", \"align\"] {\n",
    replace: "                for field in [\"align\", \"size\"] {\n",
    tests: ["test/corpus.test.ts", "-t", "^std_types_hashed_as_std_does.rs$"],
  },
];
