// Mutations of src/lower/std_types/io.rs (#1156).
import type { Mutation } from "../../../mutations";

export const mutations: Mutation[] = [
  {
    name: "deque-front-slice-read",
    breaks: "a `VecDeque<u8>`'s `read` and `fill_buf`, which read its front slice alone, a JS array has none of, are called of `$dequeReader`, which has neither, where they're refused (#1302)",
    file: "src/lower/std_types/io.rs",
    find: '                if deque && matches!(name.as_str(), "read" | "fill_buf") {',
    replace: "                if false {",
    tests: ["test/diagnostics.test.ts", "-t", "VecDeque's (read|fill_buf)"],
  },
  {
    name: "known-source-unboxed",
    breaks: "a `Cursor` whose type is known is given to its dictionary as the object, not in the box generic code gives it in",
    file: "src/lower/std_types/io.rs",
    find: "                    false => crate::lower::mut_refs::boxed(receiver),\n",
    replace: "                    false => receiver,\n",
    tests: ["test/corpus.test.ts", "-t", "^io_cursor.rs$"],
  },
  {
    name: "known-read-exact-uncaught",
    breaks: "a known source's `read_exact` whose result is a value isn't caught, so its error is thrown past `unwrap_err()`",
    file: "src/lower/std_types/io.rs",
    find: "                self.fmt_result_value(call.def_id, call.generic_args, called)\n",
    replace: "                called\n",
    tests: ["test/corpus.test.ts", "-t", "^io_read_slice.rs$"],
  },
  {
    name: "cursor-starts-at-one",
    breaks: "`Cursor::new` starts at position 1",
    file: "src/lower/std_types/io.rs",
    find: "                Prop::Field(\"pos\".into(), Expr::bigint(0)),\n",
    replace: "                Prop::Field(\"pos\".into(), Expr::bigint(1)),\n",
    tests: ["test/corpus.test.ts", "-t", "^io_cursor.rs$"],
  },
  {
    name: "read-byte-of-the-source-read",
    breaks: "`Bytes::next`'s `$readByte` is given a slice's dictionary whatever it reads, so a chain's bytes are read as its first slice's (#1156)",
    file: "src/lower/std_types/io.rs",
    find: "                Expr::call_helper(\"$readByte\", vec![reader, dictionary])\n",
    replace: "                let _ = dictionary;\n                Expr::call_helper(\"$readByte\", vec![reader, Expr::helper(\"$sliceReader\")])\n",
    tests: ["test/std-package.test.ts", "-t", "the committed @rust-js/std is std"],
  },
];
