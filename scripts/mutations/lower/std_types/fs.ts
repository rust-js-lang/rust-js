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
];
