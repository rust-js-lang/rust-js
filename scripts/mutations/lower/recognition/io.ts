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
    find: "        \"std::io::read::append_to_string\" => Some(IoOp::AppendToString),\n",
    replace: "        \"std::io::read::append_to_strings\" => Some(IoOp::AppendToString),\n",
    tests: ["test/std-package.test.ts", "-t", "the committed @rust-js/std is std"],
  },
  {
    name: "read-byte-compiled",
    breaks: "alloc's `inlined_slow_read_byte`, `Bytes::next`'s, is compiled, not answered, and its `slice::from_mut` refused (#1156)",
    file: "src/lower/recognition/io.rs",
    find: "        \"std::io::util::inlined_slow_read_byte\" => Some(IoOp::ReadByte),\n",
    replace: "        \"std::io::util::inlined_slow_read_bytes\" => Some(IoOp::ReadByte),\n",
    tests: ["test/std-package.test.ts", "-t", "the committed @rust-js/std is std"],
  },
  {
    name: "take-no-std-reader",
    breaks: "a `Take` is none of std's readers of others: its `Read` is refused as a reader of the crate's own (#1156)",
    file: "src/lower/recognition/io.rs",
    find: "        if matches!(std_path(tcx, adt.did()), \"std::io::Chain\" | \"std::io::Take\")\n",
    replace: "        if matches!(std_path(tcx, adt.did()), \"std::io::Chain\")\n",
    tests: ["test/corpus.test.ts", "-t", "^io_read_chain_take_bytes.rs$"],
  },
  {
    name: "std-reader-own-methods-compiled",
    breaks: "a known `Chain`'s or `Take`'s `read_exact`, which its impl doesn't write, is called as std's provided body, not its dictionary's `$readerDefaults` (#1156)",
    file: "src/lower/recognition/io.rs",
    find: "                    && DICTIONARY_METHODS.contains(&name.as_str()))\n                || own_reader))",
    replace: "                    && false)\n                || own_reader))",
    tests: ["test/corpus.test.ts", "-t", "^io_read_chain_take_bytes.rs$"],
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
    find: "is_io_reading(tcx, trait_id)) || buf_reader_seek_relative",
    replace: "is_io_reading(tcx, trait_id) && false) || buf_reader_seek_relative",
    tests: ["test/corpus.test.ts", "-t", "^io_read_slice.rs$"],
  },
  {
    name: "cursor-position-is-its-inner",
    breaks: "`c.position()` reads the cursor's `inner`",
    file: "src/lower/recognition/io.rs",
    find: "                \"position\" => Some(IoOp::CursorPosition),\n",
    replace: "                \"position\" => Some(IoOp::Inner),\n",
    tests: ["test/corpus.test.ts", "-t", "^io_cursor.rs$"],
  },
  {
    name: "empty-no-byte-source",
    breaks: "an `io::Empty` isn't one of std's byte sources, so its reads are refused (#1156)",
    file: "src/lower/recognition/io.rs",
    find: '                "std::io::Empty" => Some(ByteSource::Empty),\n',
    replace: "",
    tests: ["test/corpus.test.ts", "-t", "^io_read_empty_and_repeat\\.rs$"],
  },
  {
    name: "repeat-made-unanswered",
    breaks: "`io::repeat(b)` is std's function, called, which rust-js refuses (#1156)",
    file: "src/lower/recognition/io.rs",
    find: '        "std::io::repeat" => Some(IoOp::Repeat),\n',
    replace: "",
    tests: ["test/corpus.test.ts", "-t", "^io_read_empty_and_repeat\\.rs$"],
  },
  {
    name: "empty-own-lines-unwritten",
    breaks: "`read_line` and `read_until` of an `io::Empty`, which std's impl writes itself, are taken as provided by `BufRead`, its impl's method, which rust-js refuses to call (#1156)",
    file: "src/lower/recognition/io.rs",
    find: '            || (self == ByteSource::Empty && matches!(name, "read_until" | "skip_until" | "read_line"))\n',
    replace: "",
    tests: ["test/corpus.test.ts", "-t", "^io_read_empty_and_repeat\\.rs$"],
  },
  {
    name: "buf-reader-no-byte-source",
    breaks: "a `BufReader` is none of std's byte sources: its reads are refused as a reader of the crate's own (#1156)",
    file: "src/lower/recognition/io.rs",
    find: "            ty::Adt(adt, args) if is_buf_reader(self.tcx, adt.did()) => Some(ByteSource::BufReader(args.type_at(0))),\n",
    replace: "",
    tests: ["test/corpus.test.ts", "-t", "^io_buf_reader.rs$"],
  },
  {
    name: "buf-reader-new-unanswered",
    breaks: "`BufReader::new` is std's function, called, which rust-js refuses (#1156)",
    file: "src/lower/recognition/io.rs",
    find: "                    \"new\" => Some(IoOp::BufReaderNew),\n",
    replace: "",
    tests: ["test/corpus.test.ts", "-t", "^io_buf_reader.rs$"],
  },
  {
    name: "buf-reader-with-capacity-as-new",
    breaks: "`BufReader::with_capacity(n, r)` makes a buffer of the default size, so `capacity()` says 8192 (#1156)",
    file: "src/lower/recognition/io.rs",
    find: "\"with_capacity\" => Some(IoOp::BufReaderWithCapacity),",
    replace: "\"with_capacity\" => Some(IoOp::BufReaderNew),",
    tests: ["test/corpus.test.ts", "-t", "^io_buf_reader.rs$"],
  },
  {
    name: "buf-reader-capacity-is-buffer",
    breaks: "`r.capacity()` is the length of what's buffered, `buffer().len()`, not the buffer's (#1156)",
    file: "src/lower/recognition/io.rs",
    find: "                    \"capacity\" => Some(IoOp::BufReaderCapacity),\n",
    replace: "                    \"capacity\" => Some(IoOp::BufReaderBuffer),\n",
    tests: ["test/corpus.test.ts", "-t", "^io_buf_reader.rs$"],
  },
  {
    name: "buf-reader-get-ref-unanswered",
    breaks: "`BufReader::get_ref`, `get_mut` and `into_inner` are std's functions, called, which rust-js refuses (#1156)",
    file: "src/lower/recognition/io.rs",
    find: "\"with_capacity\" => Some(IoOp::BufReaderWithCapacity),\n                    \"get_ref\" | \"get_mut\" | \"into_inner\" => Some(IoOp::Inner),\n",
    replace: "\"with_capacity\" => Some(IoOp::BufReaderWithCapacity),\n",
    tests: ["test/corpus.test.ts", "-t", "^io_buf_reader.rs$"],
  },
  {
    name: "buf-reader-seek-relative-unanswered",
    breaks: "`BufReader::seek_relative`, the inherent method, is std's function, called, which rust-js refuses (#1156)",
    file: "src/lower/recognition/io.rs",
    find: "                    \"seek_relative\" => Some(IoOp::Dictionary),\n",
    replace: "",
    tests: ["test/corpus.test.ts", "-t", "^io_buf_reader.rs$"],
  },
  {
    name: "seek-relative-not-in-dictionary",
    breaks: "`seek_relative` is no method a source's dictionary holds, so a `Cursor`'s is refused as std's provided one, which `@rust-js/std` no longer compiles (#1156)",
    file: "src/lower/recognition/io.rs",
    find: "    \"seek_relative\",\n];",
    replace: "];",
    tests: ["test/corpus.test.ts", "-t", "^io_cursor.rs$"],
  },
  {
    name: "buf-reader-seek-relative-never-fails",
    breaks: "`BufReader::seek_relative` is no `io::Result<()>` that may fail, so its error is thrown past `unwrap_err()` (#1156)",
    file: "src/lower/recognition/io.rs",
    find: "|| buf_reader_seek_relative(tcx, id)\n",
    replace: "|| (buf_reader_seek_relative(tcx, id) && false)\n",
    tests: ["test/corpus.test.ts", "-t", "^io_buf_reader.rs$"],
  },
  {
    name: "file-no-reader",
    breaks: "a `File` isn't one of std's byte sources, so its reads are refused, as a reader of the crate's own (#1156)",
    file: "src/lower/recognition/io.rs",
    find: "        if is_file_handle(self.tcx, ty) {\n            return Some(ByteSource::File);\n        }\n",
    replace: "",
    tests: ["test/corpus.test.ts", "-t", "^fs_file.rs$"],
  },
  {
    name: "file-writes-unrecognized",
    breaks: "a `File`'s `write_all` isn't its dictionary's, `$fileWriter`'s, and is refused (#1156)",
    file: "src/lower/recognition/io.rs",
    find: "            return (is_file_handle(tcx, self_ty) && FILE_WRITES.contains(&name.as_str())).then_some(IoOp::Dictionary);",
    replace: "            return None;",
    tests: ["test/corpus.test.ts", "-t", "^fs_file.rs$"],
  },
  {
    name: "seek-relative-asked-of-a-closures-name",
    breaks:
      "whether a closure that may fail is `BufReader::seek_relative` asks its name, which it has none of: rustc panics, `item_name: no name`, building a library whose closure formats a `dyn` (#1392)",
    file: "src/lower/recognition/io.rs",
    find: "    tcx.opt_item_name(id)\n        .is_some_and(|name| name.as_str() == \"seek_relative\")\n",
    replace: "    tcx.item_name(id).as_str() == \"seek_relative\"\n",
    tests: ["test/corpus.test.ts", "-t", "^(closure_writes_captured_formatter|rc_dropped_in_a_closure_where_callers_unseen).rs$"],
  },
  {
    name: "shared-file-no-handle",
    breaks: "a `&File` isn't a file's handle, so `(&file).write_all(..)` is refused (#1397)",
    file: "src/lower/recognition/io.rs",
    find: "        ty::Ref(_, file, Mutability::Not) => is_file(tcx, *file),\n",
    replace: "        ty::Ref(_, file, Mutability::Not) => is_file(tcx, *file) && false,\n",
    tests: ["test/corpus.test.ts", "-t", "^fs_file_shared.rs$"],
  },
  {
    name: "shared-file-no-byte-source",
    breaks: "a `&File` is no byte source, so a `BufReader<&File>` and a `&File`'s `read_exact` are refused (#1397)",
    file: "src/lower/recognition/io.rs",
    find: "        if is_file_handle(self.tcx, ty) {\n            return Some(ByteSource::File);\n",
    replace: "        if is_file(self.tcx, ty) {\n            return Some(ByteSource::File);\n",
    tests: ["test/corpus.test.ts", "-t", "^fs_file_shared.rs$"],
  },
  {
    name: "reads-seen-own-maybe",
    breaks: "a crate's own reader is said to show its reads to no one, so its `read_to_string` into a `String` not fresh compiles, refused as it runs (#1297)",
    file: "src/lower/recognition/io.rs",
    find: "        return true;\n    }\n    let sources",
    replace: "        return false;\n    }\n    let sources",
    tests: [
      "test/diagnostics.test.ts",
      "-t",
      "String with capacity"
    ]
  },
  {
    name: "reads-seen-sources-unread",
    breaks: "a `Take` of a crate's reader is said to show its reads to no one, so its `read_to_string` into a `String` not fresh compiles (#1297)",
    file: "src/lower/recognition/io.rs",
    find: "        Some((_, sources)) => sources,\n",
    replace: "        Some(_) => return false,\n",
    tests: [
      "test/diagnostics.test.ts",
      "-t",
      "Take of a crate reader"
    ]
  },
  {
    name: "reads-to-string-never",
    breaks: "no call is `read_to_string`, so none is told its `String` is fresh-empty and a crate's reader read into one is refused (#1297)",
    file: "src/lower/recognition/io.rs",
    find: "        && tcx.item_name(id).as_str() == \"read_to_string\"\n",
    replace: "        && false\n",
    tests: [
      "test/corpus.test.ts",
      "-t",
      "^read_of_a_crates_own_reader.rs$"
    ]
  },
  {
    name: "own-reader-provided-not-dictionary",
    breaks: "a crate's reader's `read_to_string`, `read_to_end` and `read_exact` aren't its dictionary's, and are refused (#1297)",
    file: "src/lower/recognition/io.rs",
    find: "                || own_reader))\n",
    replace: "))\n",
    tests: [
      "test/corpus.test.ts",
      "-t",
      "^read_of_a_crates_own_reader.rs$"
    ]
  },
  {
    name: "empty-string-default-unmade",
    breaks: "`String::default()` isn't taken as making an empty `String`, so a crate's reader read into one is refused (#1297)",
    file: "src/lower/recognition/io.rs",
    find: "            .is_some_and(|tr| is_std_def(tcx, tr, StdItem::Default))\n",
    replace: "            .is_some_and(|_| false)\n",
    tests: [
      "test/corpus.test.ts",
      "-t",
      "^read_of_a_crates_own_reader.rs$"
    ]
  },
  {
    name: "own-io-provided-not-dictionary",
    breaks: "a crate type's call of `Seek`'s provided `stream_position` isn't its dictionary's, and is refused (#1489)",
    file: "src/lower/recognition/io.rs",
    find: "            && DICTIONARY_METHODS.contains(&name.as_str());\n",
    replace: "            && matches!(name.as_str(), \"read_exact\" | \"read_to_end\" | \"read_to_string\");\n",
    tests: [
      "test/corpus.test.ts",
      "-t",
      "^readers_of_the_crates_own_and_boxed.rs$"
    ]
  },
];
