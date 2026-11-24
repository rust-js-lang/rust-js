// `env::args()` is the process's arguments, Node's, which a browser hasn't
// (ADR 0391): its program's path, then what it's given, here nothing, as
// its native binary is given none.
use std::env;

fn counted(args: env::Args) -> usize {
    args.count()
}

fn main() {
    let args: Vec<String> = env::args().collect();
    println!("{} {}", args.len(), env::args().len());
    println!("{}", args[0].is_empty());
    for arg in env::args().skip(1) {
        println!("{arg}");
    }
    println!("{:?}", env::args().nth(5));
    let mut rest = env::args();
    let program = rest.next();
    println!("{} {} {}", program.is_some(), rest.len(), counted(env::args()));
}
//@ nondeterministic: its run reads its arguments
