// Mutations of src/lower/pointers/casts.rs, a cast of or to a raw pointer (ADR 0407).
import type { Mutation } from "../../../mutations";

export const mutations: Mutation[] = [
  {
    name: "pointer-cast-any",
    breaks: "a pointer cast to another type's is taken, read as that type",
    file: "src/lower/pointers/casts.rs",
    find: '            _ => Err(refuse(\n                self.tcx,\n                span,\n                Refused::Memory("a pointer read as another type\'s"),\n            )),\n',
    replace: "            _ => Ok(pointer),\n",
    tests: [
      "test/diagnostics.test.ts",
      "-t",
      "a struct's pointer as a later field's",
    ],
  },
  {
    name: "pointer-null-of-address",
    breaks: "`3 as *const T` is `null`",
    file: "src/lower/pointers/casts.rs",
    find: "            mir::CastKind::PointerWithExposedProvenance if self.is_handle_pointer(to) => match value.as_int() {\n                Some(0) => Ok(Some(Expr::null())),\n",
    replace:
      "            mir::CastKind::PointerWithExposedProvenance if self.is_handle_pointer(to) => match value.as_int() {\n                Some(_) => Ok(Some(Expr::null())),\n",
    tests: ["test/corpus.test.ts", "-t", "^pointer_addresses.rs$"],
  },
  {
    name: "pointer-address-not-taken",
    breaks: "`p as usize` isn't its address but a cast of the handle, `NaN`",
    file: "src/lower/pointers/casts.rs",
    find: "            mir::CastKind::PointerExposeProvenance if self.is_handle_pointer(from) => {\n",
    replace:
      "            mir::CastKind::PointerExposeProvenance if false => {\n",
    tests: ["test/corpus.test.ts", "-t", "^pointer_addresses.rs$"],
  },
  {
    name: "recast-first-item-refused",
    breaks:
      "an array's pointer cast to its item's, arrayvec's `self.xs.as_ptr() as *const T`, is read through the array's bytes, not `$ptr(xs, 0)`",
    file: "src/lower/pointers/casts.rs",
    find: "            (ty::Array(item, _), _) if self.same_place(*item, b) => Ok(self.first_item_pointer(pointer)),\n",
    replace: "",
    tests: ["test/corpus.test.ts", "-t", "^array_vec_raw.rs$"],
    snapshots: true,
  },
  {
    name: "recast-uninit-not-its-value",
    breaks:
      "a `MaybeUninit<T>`'s pointer cast to a `T`'s is refused, though JS holds it as its value",
    file: "src/lower/pointers/casts.rs",
    find: "        let value = |t: Ty<'tcx>| self.recognition().uninit_of(t).unwrap_or(t);\n        let (a, b) = (value(a), value(b));\n",
    replace: "        let value = |t: Ty<'tcx>| t;\n        let (a, b) = (value(a), value(b));\n",
    tests: [
      "test/corpus.test.ts",
      "-t",
      "^(array_vec_raw|bytes_through_pointers).rs$",
    ],
    snapshots: true,
  },
  {
    name: "recast-arrays-apart",
    breaks:
      "an array of `T`s' pointer cast to one of `MaybeUninit<T>`s, arrayvec's, is refused",
    file: "src/lower/pointers/casts.rs",
    find: "            (ty::Array(x, n), ty::Array(y, m)) => n == m && self.same_place(*x, *y),\n",
    replace:
      "            (ty::Array(x, n), ty::Array(y, m)) => false && n == m && self.same_place(*x, *y),\n",
    tests: ["test/corpus.test.ts", "-t", "^array_vec_raw.rs$"],
  },
  {
    name: "recast-whole-refused",
    breaks:
      "a first item's pointer cast to the whole array's, arrayvec's `into_inner`, is refused",
    file: "src/lower/pointers/casts.rs",
    find: "            (_, ty::Array(item, length)) if self.same_place(a, *item) && !is_bytes(a) => {\n",
    replace:
      "            (_, ty::Array(item, length)) if false && self.same_place(a, *item) && !is_bytes(a) => {\n",
    tests: ["test/corpus.test.ts", "-t", "^array_vec_raw.rs$"],
  },
  {
    name: "recast-generic-refused",
    breaks:
      "a cast in generic code, arrayvec's `ptr.cast::<u8>()` of a type with no size, is refused for every type",
    file: "src/lower/pointers/casts.rs",
    find: '            _ if a.has_param() || b.has_param() => Ok(helper("$ptrRecast", vec![pointer])),\n',
    replace: "",
    tests: ["test/corpus.test.ts", "-t", "^maybe_uninit_pointer.rs$"],
  },
  {
    name: "slice-pointer-cast-unread",
    breaks:
      "a slice pointer cast to its item's is the slice, not its first item's handle, so writes through it go nowhere",
    file: "src/lower/pointers/casts.rs",
    find: "                ) && self.same_place(item, b) =>\n",
    replace: "                ) && false =>\n",
    tests: ["test/corpus.test.ts", "-t", "^slice_pointers.rs$"],
  },
  {
    name: "to-bytes-refused",
    breaks:
      "a pointer cast to bytes, an allocation's given to `realloc` or `dealloc`, is refused",
    file: "src/lower/pointers/casts.rs",
    find: '            (_, _) if is_bytes(b) => Ok(match self.plain_codec(a, span)? {\n                Some((size, encode, decode)) => helper("$ptrBytes", vec![pointer, size, encode, decode]),\n                None => helper("$ptrBytes", vec![pointer]),\n            }),\n',
    replace: "",
    tests: ["test/corpus.test.ts", "-t", "^allocator.rs$"],
  },
  {
    name: "from-bytes-refused",
    breaks: "an allocation's pointer to bytes cast to its items' is refused",
    file: "src/lower/pointers/casts.rs",
    find: '            (_, _) if is_bytes(a) => Ok(helper("$ptrTyped", vec![pointer])),\n',
    replace: "",
    tests: ["test/corpus.test.ts", "-t", "^allocator.rs$"],
  },
  {
    name: "array-item-pointer-recast",
    breaks:
      "smallvec's `*const A` as `*const A::Item`, a whole array's pointer as its first item's, is rust-js's error where it's made",
    file: "src/lower/pointers/casts.rs",
    find: '            _ if self.is_array_item_of(b, a) => Ok(helper("$ptrFirst", vec![pointer])),\n',
    replace: "",
    tests: ["test/corpus.test.ts", "-t", "^maybe_uninit_pointer.rs$"],
  },
  {
    name: "uninit-slice-cast-refused",
    breaks:
      "a `*mut [MaybeUninit<T>]` cast to a `*mut [T]` is refused, where it's the slice itself",
    file: "src/lower/pointers/casts.rs",
    find: "                    && let (ty::Slice(a), ty::Slice(b)) = (a.kind(), b.kind())\n                    && self.same_place(*a, *b) =>\n",
    replace:
      "                    && let (ty::Slice(a), ty::Slice(b)) = (a.kind(), b.kind())\n                    && false && self.same_place(*a, *b) =>\n",
    tests: ["test/corpus.test.ts", "-t", "^uninit_slice_cast.rs$"],
  },
  {
    name: "fn-pointer-cast-numeric",
    breaks:
      "a function cast to a pointer goes through the numeric cast, refused",
    file: "src/lower/pointers/casts.rs",
    find: "            mir::CastKind::FnPtrToPtr if self.is_handle_pointer(to) => Ok(Some(value)),\n",
    replace: "",
    tests: ["test/corpus.test.ts", "-t", "^pointer_addresses.rs$"],
  },
  {
    name: "recast-bytes-opaque",
    breaks: "plain data cast to bytes is a view that refuses every read",
    file: "src/lower/pointers/casts.rs",
    find: '                Some((size, encode, decode)) => helper("$ptrBytes", vec![pointer, size, encode, decode]),',
    replace: '                Some(_) => helper("$ptrBytes", vec![pointer]),',
    tests: ["test/corpus.test.ts", "-t", "^bytes_through_pointers.rs$"],
  },
  {
    name: "retyped-unsigned",
    breaks: "bytes read as an `i8` are read unsigned",
    file: "src/lower/pointers/casts.rs",
    find: "                let signed = Expr::bool(b.is_signed());",
    replace: "                let signed = Expr::bool(false);",
    tests: ["test/corpus.test.ts", "-t", "^bytes_through_pointers.rs$"],
  },
  {
    name: "dyn-pointer-thin-refused",
    breaks: "a `*const dyn Trait` made thin, `z as *const A`, is refused",
    file: "src/lower/pointers/casts.rs",
    find: "                Ok(Some(self.pointer_to(\n                    Expr::member(value, js::HELD),\n                    b,\n                    Offset::Unsaid,\n                    span,\n                )?))\n",
    replace:
      '                Err(refuse(self.tcx, span, Refused::Memory("a pair\'s pointer made thin")))\n',
    tests: ["test/corpus.test.ts", "-t", "^dyn_pointers.rs$"],
  },
  {
    name: "first-field-pointer-refused",
    breaks:
      "a struct's pointer cast to its first field's type is refused, where rustc lays the field at offset 0",
    file: "src/lower/pointers/casts.rs",
    find: "                if adt.is_struct()\n                    && let Some(field) = self.field_at_zero(a, args, b) =>\n",
    replace:
      "                if false\n                    && let Some(field) = self.field_at_zero(a, args, b) =>\n",
    tests: ["test/corpus.test.ts", "-t", "^pointer_relayout.rs$"],
  },
  {
    name: "prefix-cast-refused",
    breaks:
      "a `#[repr(C)]` struct's pointer cast to another layout of the struct, `ErrorImpl<()>` as `ErrorImpl<E>`, is refused where it runs",
    file: "src/lower/pointers/casts.rs",
    find: "            (ty::Adt(x, _), ty::Adt(y, _)) if x.did() == y.did() && x.is_struct() && x.repr().c() => Ok(pointer),\n",
    replace: "",
    tests: ["test/corpus.test.ts", "-t", "^pointer_relayout.rs$"],
  },
  {
    name: "same-place-minds-lifetimes",
    breaks:
      "a `&'static V` field isn't a `&V` in JS, so a struct's pointer cast to its reference field's type is refused",
    file: "src/lower/pointers/casts.rs",
    find: "            self.tcx\n                .erase_and_anonymize_regions(self.recognition().uninit_of(t).unwrap_or(t))\n",
    replace: "            self.recognition().uninit_of(t).unwrap_or(t)\n",
    tests: ["test/corpus.test.ts", "-t", "^pointer_relayout.rs$"],
  },
  {
    name: "field-at-zero-any-offset",
    breaks:
      "a struct's pointer is cast to any field's type, not the one at offset 0",
    file: "src/lower/pointers/casts.rs",
    find: "            let placed = layout.fields.offset(index.index()).bytes() == 0;\n",
    replace: "            let placed = true;\n",
    tests: [
      "test/diagnostics.test.ts",
      "-t",
      "a struct's pointer as a later field's",
    ],
  },
  {
    name: "signed-bytes-are-bytes",
    breaks: "a pointer to `i8`s is taken for one to bytes, so an `i8` array read through a `*const u8` keeps its sign, `-21` where native Rust's is `235` (#1253)",
    file: "src/lower/pointers/casts.rs",
    find: "        let is_bytes = |t: Ty<'tcx>| matches!(t.kind(), ty::Uint(ty::UintTy::U8));\n",
    replace: "        let is_bytes = |t: Ty<'tcx>| matches!(t.kind(), ty::Uint(ty::UintTy::U8) | ty::Int(ty::IntTy::I8));\n",
    tests: ["test/corpus.test.ts", "-t", "^pointer_cast_to_the_other_sign.rs$"],
  },
  {
    name: "slice-pointer-as-an-array-pointer-refused",
    breaks:
      "a slice's pointer cast to an array's of its items, `input as *const [u8] as *const [u8; 16]`, is refused as addressable memory: ppv-lite86's `unsafe_read_le` (#1402)",
    file: "src/lower/pointers/casts.rs",
    find: "                ) && matches!(b.kind(), ty::Array(of, _) if self.same_place(item, *of)) =>\n",
    replace: "                ) && false =>\n",
    tests: ["test/corpus.test.ts", "-t", "^lanes_read_through_a_byte_array_pointer.rs$"],
  },
];
