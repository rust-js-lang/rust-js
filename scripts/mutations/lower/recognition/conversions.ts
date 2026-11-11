// Mutations of src/lower/recognition/conversions.rs, split from src/lower/recognition/calls.rs by concept.
import type { Mutation } from "../../../mutations";

export const mutations: Mutation[] = [
  {
    name: "box-from-unknown",
    breaks: "`22.into()` to a `Box` is refused",
    file: "src/lower/recognition/conversions.rs",
    find: "            if let ty::Adt(_, boxed) = to_ty.kind()\n                && to_ty.is_box()",
    replace:
      "            if let ty::Adt(_, boxed) = to_ty.kind()\n                && false",
    tests: ["test/corpus.test.ts", "-t", "std_odds"],
  },
  {
    name: "path-from-text-unknown",
    breaks: "`PathBuf::from(text)` is an error",
    file: "src/lower/recognition/conversions.rs",
    find: "            if (self.is_lang_adt(to_ty, LangItem::String) || self.is_path_like(to_ty) || boxed_str)",
    replace:
      "            if (self.is_lang_adt(to_ty, LangItem::String) || boxed_str)",
    tests: ["test/corpus.test.ts", "-t", "path_text"],
  },
  {
    name: "boxed-str-unconverted",
    breaks: "chrono's `Box::from(s)` of a `&str` is refused",
    file: "src/lower/recognition/conversions.rs",
    find: "            let boxed_str = to_ty.is_box() && to_ty.boxed_ty().is_some_and(|inner| inner.is_str());",
    replace: "            let boxed_str = false;",
    tests: ["test/corpus.test.ts", "-t", "boxed_str"],
  },
  {
    name: "number-from-bool-unrecognized",
    breaks: "`usize::from(flag)` and `u32::from(c)` are refused, which std has",
    file: "src/lower/recognition/conversions.rs",
    find: "            if (from_ty.is_bool() || from_ty.is_char()) && Num::of(to_ty).is_some() {",
    replace:
      "            if false && (from_ty.is_bool() || from_ty.is_char()) && Num::of(to_ty).is_some() {",
    tests: ["test/lowering-numbers.test.ts", "-t", "from a bool or a char"],
  },
  {
    name: "number-into-as-from",
    breaks:
      "`flag.into()` takes its target for its source, a `bool` for the `u64`",
    file: "src/lower/recognition/conversions.rs",
    find: "                return Some(Std::Cast { into });\n",
    replace: "                return Some(Std::Cast { into: false });\n",
    tests: ["test/lowering-numbers.test.ts", "-t", "from a bool or a char"],
  },
  {
    name: "conversion-to-itself-unknown",
    breaks:
      "`From::from(e)` of an error to its own type, and `.into()` of an `impl Into<T>` that is a `T`, are refused",
    file: "src/lower/recognition/conversions.rs",
    find: "        if tcx.erase_and_anonymize_regions(self.reveal(from_ty)) == tcx.erase_and_anonymize_regions(self.reveal(to_ty))\n",
    replace: "        if false\n",
    tests: ["test/corpus.test.ts", "-t", "^(into_itself|generic_into).rs$"],
  },
  {
    name: "option-from-unknown",
    breaks: "`Option::from(x)` and `x.into()` of one are refused",
    file: "src/lower/recognition/conversions.rs",
    find: "        if self.option_of(to_ty) == Some(from_ty) {\n",
    replace: "        if false && self.option_of(to_ty) == Some(from_ty) {\n",
    tests: ["test/corpus.test.ts", "-t", "^std_conversions.rs$"],
  },
  {
    name: "vec-from-array-unknown",
    breaks: "`Vec::from([..])` and `[..].into()` of one are refused",
    file: "src/lower/recognition/conversions.rs",
    find: "            from_ty.is_array() || from_ty.peel_refs().is_slice() || self.is_std_type(from_ty, StdItem::VecDeque);\n",
    replace:
      "            from_ty.peel_refs().is_slice() || self.is_std_type(from_ty, StdItem::VecDeque);\n",
    tests: ["test/corpus.test.ts", "-t", "^std_conversions.rs$"],
  },
  {
    name: "rc-from-unknown",
    breaks: "`Rc::from(x)` and `x.into()` of one are refused",
    file: "src/lower/recognition/conversions.rs",
    find: "        if rc_made_of(tcx, from_ty, to_ty).is_some() {\n",
    replace:
      "        if false && rc_made_of(tcx, from_ty, to_ty).is_some() {\n",
    tests: ["test/corpus.test.ts", "-t", "^std_conversions.rs$"],
  },
  {
    name: "try-from-as-from",
    breaks:
      "`try_from` between integers is taken for `from`, the value itself, unchecked",
    file: "src/lower/recognition/conversions.rs",
    find: "        if !into && !tcx.is_diagnostic_item(sym::From, trait_) {\n            return None;\n        }\n",
    replace: "",
    tests: ["test/corpus.test.ts", "-t", "^(integers_128|nonzero).rs$"],
  },
  {
    name: "cow-from-unknown",
    breaks: "`Cow::from(x)` and `x.into()` of one are refused",
    file: "src/lower/recognition/conversions.rs",
    find: "            if of_owned || lent {\n",
    replace: "            if false && (of_owned || lent) {\n",
    tests: ["test/corpus.test.ts", "-t", "^std_conversions.rs$"],
  },
  {
    name: "string-from-anything-same",
    breaks:
      "a `String` from the crate's own type, by its `From<W> for String`, is the value itself, `[object Object]`",
    file: "src/lower/recognition/conversions.rs",
    find: "            if (self.is_lang_adt(to_ty, LangItem::String) || self.is_path_like(to_ty) || boxed_str)\n                && self.is_string_like(from_ty)\n",
    replace:
      "            if (self.is_lang_adt(to_ty, LangItem::String) || self.is_path_like(to_ty) || boxed_str)\n",
    tests: ["test/corpus.test.ts", "-t", "^string_from_own.rs$"],
  },
  {
    name: "string-from-boxed-str-refused",
    breaks: "`String::from` of a `Box<str>` is refused",
    file: "src/lower/recognition/conversions.rs",
    find: "            if from_ty.is_box() && from_ty.boxed_ty().is_some_and(|text| text.is_str()) {\n",
    replace: "            if false {\n",
    tests: ["test/corpus.test.ts", "-t", "^string_from_own.rs$"],
  },
  {
    name: "string-from-cow-refused",
    breaks: "`String::from` of a `Cow<str>` is refused",
    file: "src/lower/recognition/conversions.rs",
    find: "            if self.cow_parts(from_ty).is_some_and(|(borrowed, _)| borrowed.is_str()) {\n",
    replace: "            if false {\n",
    tests: ["test/corpus.test.ts", "-t", "^string_from_own.rs$"],
  },
  {
    name: "char-try-from-refused",
    breaks: "`char::try_from(n)` of a `u32` is refused",
    file: "src/lower/recognition/conversions.rs",
    find: "        (ty::Uint(ty::UintTy::U32), ty::Char) => Some(Std::TryFromChar { to_char: true }),\n",
    replace: "",
    tests: ["test/corpus.test.ts", "-t", "^char_try_from.rs$"],
  },
  {
    name: "vec-from-slice-refused",
    breaks: "`Vec::from(&v[..])` is refused, a slice not taken for items",
    file: "src/lower/recognition/conversions.rs",
    find: "            from_ty.is_array() || from_ty.peel_refs().is_slice() || self.is_std_type(from_ty, StdItem::VecDeque);\n",
    replace:
      "            from_ty.is_array() || self.is_std_type(from_ty, StdItem::VecDeque);\n",
    tests: ["test/corpus.test.ts", "-t", "^vec_from_items.rs$"],
  },
  {
    name: "vec-from-text-refused",
    breaks: "`Vec::<u8>::from(s)` of text is refused",
    file: "src/lower/recognition/conversions.rs",
    find: "        if vec && (from_ty.peel_refs().is_str() || self.is_lang_adt(from_ty, LangItem::String)) {\n",
    replace:
      "        if false && vec && (from_ty.peel_refs().is_str() || self.is_lang_adt(from_ty, LangItem::String)) {\n",
    tests: ["test/corpus.test.ts", "-t", "^vec_from_items.rs$"],
  },
  {
    name: "heap-vec-refused",
    breaks: "`Vec::from(heap)` and `heap.into()` are refused",
    file: "src/lower/recognition/conversions.rs",
    find: "        if vec && self.is_std_type(from_ty, StdItem::BinaryHeap) {\n            return Some(Std::Same);\n",
    replace: "        if false {\n            return Some(Std::Same);\n",
    tests: ["test/corpus.test.ts", "-t", "^heap_methods.rs$"],
  },
  {
    name: "vec-from-text-lent",
    breaks: "`Vec::from(s)` of text is its bytes lent, never written, where they're its own, so a write to them is an error",
    file: "src/lower/recognition/conversions.rs",
    find: "            return Some(Std::Text(TextOp::IntoBytes));",
    replace: "            return Some(Std::Text(TextOp::Bytes));",
    tests: ["test/corpus.test.ts", "-t", "^text_bytes_lent_and_owned.rs$"],
  },
];
