// Mutations of src/lower/std_types/io_error.rs (ADR 0462).
import type { Mutation } from "../../../mutations";

export const mutations: Mutation[] = [
  {
    name: "raw-os-error-as-kind",
    breaks: "`e.raw_os_error()` is the error's kind, `Some(NotFound)`, not the OS's number, `Some(2)` (#1156)",
    file: "src/lower/std_types/io_error.rs",
    find: "            IoErrorOp::RawOsError => Expr::member(values.next().expect(\"an error\"), \"code\"),",
    replace: "            IoErrorOp::RawOsError => Expr::member(values.next().expect(\"an error\"), \"kind\"),",
    tests: ["test/corpus.test.ts", "-t", "^fs_file.rs$"],
  },
  {
    name: "io-error-kind-of-its-code",
    breaks: "`e.kind()` is the OS's number, not its kind",
    file: "src/lower/std_types/io_error.rs",
    find: "            IoErrorOp::Kind => Expr::member(values.next().expect(\"an error\"), \"kind\"),",
    replace: "            IoErrorOp::Kind => Expr::member(values.next().expect(\"an error\"), \"code\"),",
    tests: ["test/corpus.test.ts", "-t", "^fs_file.rs$"],
  },
];
