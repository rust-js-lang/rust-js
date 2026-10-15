// `env::args()` is the process's arguments, Node's, which a browser hasn't
// (ADR 0391): its program's path, then what it's given, here the file its
// outcome goes to, as the native binary is given it.
use std::env;

fn counted(args: env::Args) -> usize {
    args.count()
}

fn main() {
    let args: Vec<String> = env::args().collect();
    println!("{} {}", args.len(), env::args().len());
    println!("{}", args[1].ends_with(".json"));
    for arg in env::args().skip(1) {
        println!("{}", arg.ends_with(".json"));
    }
    println!("{:?}", env::args().nth(5));
    let mut rest = env::args();
    let program = rest.next();
    println!("{} {} {}", program.is_some(), rest.len(), counted(env::args()));
}
