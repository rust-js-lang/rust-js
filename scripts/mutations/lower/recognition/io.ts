// Mutations of src/lower/recognition/io.rs (#1156).
import type { Mutation } from "../../../mutations";

export const mutations: Mutation[] = [
  {
    name: "deque-no-byte-source",
    breaks: "a `VecDeque<u8>` is none of std's byte sources, so its `read_exact` and the rest are refused, and as generic code's reader it's refused as one of the crate's own (#1156, #1302)",
    file: "src/lower/recognition/io.rs",
    find: "            ty::Adt(_, args) if self.is_std_type(ty, StdItem::VecDeque) && args.type_at(0) == self.tcx.types.u8 => {",
    replace: "            ty::Adt(_, args) if false && self.is_std_type(ty, StdItem::VecDeque) && args.type_at(0) == self.tcx.types.u8 => {",
    tests: ["test/corpus.test.ts", "-t", "^io_read_deque.rs$"],
  },
  {
    name: "append-to-string-compiled",
    breaks: "alloc's `append_to_string` is compiled, not answered, and its `String::as_mut_vec` refused",
    file: "src/lower/recognition/io.rs",
    find: "    std_path(tcx, id) == \"std::io::read::append_to_string\"\n",
    replace: "    std_path(tcx, id) == \"std::io::read::append_to_strings\"\n",
    tests: ["test/std-package.test.ts", "-t", "the committed @rust-js/std is std"],
  },
  {
    name: "cursor-not-a-source",
    breaks: "a `Cursor` is no byte source: its reads are refused, or read as another's",
    file: "src/lower/recognition/io.rs",
    find: "            ty::Adt(adt, args) if is_cursor(self.tcx, adt.did()) => Some(ByteSource::Cursor(args.type_at(0))),\n",
    replace: "",
    tests: ["test/corpus.test.ts", "-t", "^io_cursor.rs$"],
  },
  {
    name: "seek-not-in-dictionary",
    breaks: "a `Cursor`'s `seek` is no method its dictionary holds, and is refused as a crate's reader's",
    file: "src/lower/recognition/io.rs",
    find: "    \"seek\",\n",
    replace: "",
    tests: ["test/corpus.test.ts", "-t", "^io_cursor.rs$"],
  },
  {
    name: "io-reading-never-fails",
    breaks: "a byte source's `read_exact` is never one that may fail, so its error is thrown past `unwrap_err()`",
    file: "src/lower/recognition/io.rs",
    find: "    trait_id.is_some_and(|trait_id| is_io_reading(tcx, trait_id))\n}",
    replace: "    trait_id.is_some_and(|trait_id| is_io_reading(tcx, trait_id) && false)\n}",
    tests: ["test/corpus.test.ts", "-t", "^io_read_slice.rs$"],
  },
  {
    name: "cursor-position-is-its-inner",
    breaks: "`c.position()` reads the cursor's `inner`",
    file: "src/lower/recognition/io.rs",
    find: "                \"position\" => Some(IoOp::CursorPosition),\n",
    replace: "                \"position\" => Some(IoOp::CursorInner),\n",
    tests: ["test/corpus.test.ts", "-t", "^io_cursor.rs$"],
  },
];
