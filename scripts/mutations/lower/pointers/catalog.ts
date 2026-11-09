// Mutations of src/lower/pointers/catalog.rs (ADR 0093).
import type { Mutation } from "../../../mutations";

export const mutations: Mutation[] = [
  {
    name: "pointer-ops-unknown",
    breaks: "std's functions of a raw pointer are refused",
    file: "src/lower/pointers/catalog.rs",
    find: "    if let Some(&(_, op)) = POINTER_FNS.iter().find(|(of, _)| *of == path) {\n        return Some(op);\n    }\n",
    replace: "",
    tests: ["test/corpus.test.ts", "-t", "^raw_pointers.rs$"],
  },
  {
    name: "pointer-methods-unknown",
    breaks: "a raw pointer's methods, and a `NonNull`'s, are refused",
    file: "src/lower/pointers/catalog.rs",
    find: "    methods.iter().find(|(name, _)| *name == method).map(|&(_, op)| op)\n",
    replace: "    methods.iter().find(|_| false).map(|&(_, op)| op)\n",
    tests: ["test/corpus.test.ts", "-t", "^raw_pointers.rs$"],
  },
  {
    name: "offset-from-refused",
    breaks: "`p.offset_from(q)` of two pointers into one array is refused",
    file: "src/lower/pointers/catalog.rs",
    find: '    ("offset_from", PointerOp::OffsetFrom),\n    ("offset_from_unsigned", PointerOp::OffsetFrom),\n    ("as_ref", PointerOp::AsRef),\n',
    replace: '    ("as_ref", PointerOp::AsRef),\n',
    tests: ["test/corpus.test.ts", "-t", "^pointer_offset_from.rs$"],
  },
  {
    name: "uninit-as-ptr-unrecognized",
    breaks:
      "a `MaybeUninit`'s `as_mut_ptr()` isn't a pointer to its value, and is refused",
    file: "src/lower/pointers/catalog.rs",
    find: '    ("std::mem::MaybeUninit::<T>::as_mut_ptr", PointerOp::FromRef),\n',
    replace: "",
    tests: ["test/corpus.test.ts", "-t", "^maybe_uninit_pointer.rs$"],
  },
  {
    name: "wrapping-add-as-step",
    breaks:
      "`wrapping_add` steps by items within the allocation, so semver's rotation can't leave it",
    file: "src/lower/pointers/catalog.rs",
    find: '    ("wrapping_add", PointerOp::WrappingAdd { sub: false }),',
    replace: '    ("wrapping_add", PointerOp::Add),',
    tests: ["test/corpus.test.ts", "-t", "^tagged_pointers.rs$"],
  },
  {
    name: "unaligned-read-unknown",
    breaks: "`read_unaligned` is refused",
    file: "src/lower/pointers/catalog.rs",
    find: '    ("read_unaligned", PointerOp::Read),\n    ("write", PointerOp::Write),\n    ("write_volatile", PointerOp::Write),',
    replace:
      '    ("write", PointerOp::Write),\n    ("write_volatile", PointerOp::Write),',
    tests: ["test/corpus.test.ts", "-t", "^bytes_through_pointers.rs$"],
  },
  {
    name: "rc-as-ptr-refused",
    breaks:
      "`Rc::as_ptr` isn't a pointer to what the `Rc` points at, so React's `a.as_ref().map(Rc::as_ptr)` is refused",
    file: "src/lower/pointers/catalog.rs",
    find: '    ("std::rc::Rc::<T, A>::as_ptr", PointerOp::RcAsPtr),\n',
    replace: "",
    tests: ["test/corpus.test.ts", "-t", "^rc_as_ptr.rs$"],
  },
];
