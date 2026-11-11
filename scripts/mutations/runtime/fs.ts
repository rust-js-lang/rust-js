// Mutations of src/runtime/fs.js (#1156).
import type { Mutation } from "../../mutations";

const file = ["test/corpus.test.ts", "-t", "^fs_file.rs$"];

export const mutations: Mutation[] = [
  {
    name: "os-error-number-negated",
    breaks: "an error of the OS's keeps Node's negated number, `os error -2`, where std's is the OS's, 2",
    file: "src/runtime/fs.js",
    find: "  return new $IoError(kind, undefined, platform === \"darwin\" && darwin !== undefined ? darwin : message, -e.errno);",
    replace: "  return new $IoError(kind, undefined, platform === \"darwin\" && darwin !== undefined ? darwin : message, e.errno);",
    tests: file,
  },
  {
    name: "bad-descriptor-categorized",
    breaks: "`EBADF`, a write to a file opened to read, is `Other`, where std maps it to no kind, `Uncategorized`",
    file: "src/runtime/fs.js",
    find: "  EBADF: [\"Uncategorized\", \"Bad file descriptor\"],",
    replace: "  EBADF: [\"Other\", \"Bad file descriptor\"],",
    tests: file,
  },
  {
    name: "seek-before-start-allowed",
    breaks: "a seek before a file's start moves it there, where `lseek` refuses it, `EINVAL`",
    file: "src/runtime/fs.js",
    find: "      if (n < 0n || n > 0x7fffffffffffffffn) throw $osFailure(\"EINVAL\", 22);",
    replace: "      if (n > 0x7fffffffffffffffn) throw $osFailure(\"EINVAL\", 22);",
    tests: file,
  },
  {
    name: "append-unremembered",
    breaks: "a file opened to append writes at its position, not at its end, so a second open's write replaces the first's",
    file: "src/runtime/fs.js",
    find: "    if (o.append) $appending.add(fd);\n",
    replace: "",
    tests: file,
  },
  {
    name: "append-position-unmoved",
    breaks: "an appended write leaves the position where it was, not at the end, where `stream_position()` is then",
    file: "src/runtime/fs.js",
    find: "      f.pos = BigInt(fs.fstatSync(f.fd).size);\n",
    replace: "",
    tests: file,
  },
  {
    name: "create-new-not-exclusive",
    breaks: "`create_new(true)` opens a file there already, where std's `O_EXCL` refuses it, `AlreadyExists`",
    file: "src/runtime/fs.js",
    find: "  const creation = o.createNew ? O_CREAT | O_EXCL :",
    replace: "  const creation = o.createNew ? O_CREAT :",
    tests: file,
  },
  {
    name: "open-truncating-reader-allowed",
    breaks: "`read(true).truncate(true)` opens the file, where std refuses it before it opens anything",
    file: "src/runtime/fs.js",
    find: "  if ((!o.write && !o.append && creating) || (o.append && o.truncate && !o.createNew)) return $openInvalid(asked);",
    replace: "  if (o.append && o.truncate && !o.createNew) return $openInvalid(asked);",
    tests: file,
  },
  {
    name: "read-position-unmoved",
    breaks: "a read leaves the file's position where it was, so the next reads the same bytes",
    file: "src/runtime/fs.js",
    find: "  f.pos += BigInt(read);\n",
    replace: "",
    tests: file,
  },
  {
    name: "read-to-string-unchecked",
    breaks: "`fs::read_to_string` of bytes that aren't UTF-8 decodes them, where std's is `InvalidData`",
    file: "src/runtime/fs.js",
    find: "  if ($utf8Check(read._0) !== undefined) return { TAG: \"Err\", _0: $invalidUtf8() };\n  return { TAG: \"Ok\", _0: $utf8Decode(read._0) };",
    replace: "  return { TAG: \"Ok\", _0: $utf8Decode(read._0) };",
    tests: file,
  },
  {
    name: "remove-file-error-unthrown",
    breaks: "`fs::remove_file` of a file that isn't there is `Ok`, its error Node's, never std's",
    file: "src/runtime/fs.js",
    find: "  $fsThrow(() => fs.unlinkSync(path));",
    replace: "  try {\n    fs.unlinkSync(path);\n  } catch {}",
    tests: file,
  },
];
