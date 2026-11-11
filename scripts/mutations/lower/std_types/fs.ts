// Mutations of src/lower/std_types/fs.rs (#1156, ADR 0425).
import type { Mutation } from "../../../mutations";

const file = ["test/corpus.test.ts", "-t", "^fs_file.rs$"];
const ownPath = ["test/corpus.test.ts", "-t", "^path_given_own_as_ref.rs$"];

export const mutations: Mutation[] = [
  {
    name: "fs-write-contents-unread",
    breaks: "`fs::write(path, \"text\")` is given the text, not its bytes, `AsRef<[u8]>`'s, so the file holds zeros",
    file: "src/lower/std_types/fs.rs",
    find: "                _ if op == FsOp::Write => self.bytes_given(value, ty, call)?,\n",
    replace: "",
    tests: file,
  },
  {
    name: "fs-options-open-path-unread",
    breaks: "`options.open(&name)` of a path of the crate's own type is given the struct, not its `as_ref()` (#1388)",
    file: "src/lower/std_types/fs.rs",
    find: "            FsOp::OptionsOpen => Some(1),\n",
    replace: "            FsOp::OptionsOpen => None,\n",
    tests: ownPath,
  },
  {
    name: "fs-create-new-as-create",
    breaks: "`create_new(true)` sets `create`, so a file there already is opened, not `AlreadyExists`",
    file: "src/lower/std_types/fs.rs",
    find: "\"std::fs::OpenOptions::create_new\" => FsOp::OptionsSet(\"createNew\"),",
    replace: "\"std::fs::OpenOptions::create_new\" => FsOp::OptionsSet(\"create\"),",
    tests: file,
  },
  {
    name: "file-metadata-unknown",
    breaks: "`f.metadata()` of a `File` is refused as a call rust-js doesn't know (#1397)",
    file: "src/lower/std_types/fs.rs",
    find: "        \"std::fs::File::metadata\" => FsOp::FileMetadata,\n",
    replace: "",
    tests: ["test/corpus.test.ts", "-t", "^fs_dirs_and_metadata.rs$"],
  },
  {
    name: "metadata-len-not-its-size",
    breaks: "`m.len()` reads a `Stats`' `length`, which it hasn't, not its `size` (#1397)",
    file: "src/lower/std_types/fs.rs",
    find: "        \"std::fs::Metadata::len\" => FsOp::Field(\"size\"),\n",
    replace: "        \"std::fs::Metadata::len\" => FsOp::Field(\"length\"),\n",
    tests: ["test/corpus.test.ts", "-t", "^fs_dirs_and_metadata.rs$"],
  },
  {
    name: "metadata-is-dir-as-is-file",
    breaks: "`m.is_dir()` asks `isFile()` (#1397)",
    file: "src/lower/std_types/fs.rs",
    find: "        \"std::fs::Metadata::is_dir\" => FsOp::StatsMethod(\"isDirectory\"),\n",
    replace: "        \"std::fs::Metadata::is_dir\" => FsOp::StatsMethod(\"isFile\"),\n",
    tests: ["test/corpus.test.ts", "-t", "^fs_dirs_and_metadata.rs$"],
  },
  {
    name: "file-type-is-dir-as-file",
    breaks: "`t.is_dir()` of a `FileType` asks whether it's a file's (#1397)",
    file: "src/lower/std_types/fs.rs",
    find: "        \"std::fs::FileType::is_dir\" => FsOp::FileTypeIs(\"dir\"),\n",
    replace: "        \"std::fs::FileType::is_dir\" => FsOp::FileTypeIs(\"file\"),\n",
    tests: ["test/corpus.test.ts", "-t", "^fs_dirs_and_metadata.rs$"],
  },
  {
    name: "entry-path-as-its-name",
    breaks: "`entry.path()` gives its name alone, not the directory's joined with it (#1397)",
    file: "src/lower/std_types/fs.rs",
    find: "        \"std::fs::DirEntry::path\" => FsOp::Field(\"path\"),\n",
    replace: "        \"std::fs::DirEntry::path\" => FsOp::Field(\"name\"),\n",
    tests: ["test/corpus.test.ts", "-t", "^fs_dirs_and_metadata.rs$"],
  },
  {
    name: "file-type-kind-unequal",
    breaks: "`t.is_dir()` of a `FileType` is whether it isn't one (#1397)",
    file: "src/lower/std_types/fs.rs",
    find: "                    Called::Kind(kind) => Expr::bin(crate::js::Op::Eq, of, Expr::str(kind)),\n",
    replace: "                    Called::Kind(kind) => Expr::bin(crate::js::Op::Ne, of, Expr::str(kind)),\n",
    tests: ["test/corpus.test.ts", "-t", "^fs_dirs_and_metadata.rs$"],
  },
  {
    name: "entry-type-not-ok",
    breaks: "`entry.file_type()` gives the type, not `Ok` of it, which `?` reads as an `Err` (#1397)",
    file: "src/lower/std_types/fs.rs",
    find: "                    Called::OkOf(field) => crate::js::result::ok(Expr::member(of, field)),\n",
    replace: "                    Called::OkOf(field) => Expr::member(of, field),\n",
    tests: ["test/corpus.test.ts", "-t", "^fs_dirs_and_metadata.rs$"],
  },
];
