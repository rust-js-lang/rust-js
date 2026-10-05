use bitflags::bitflags;

bitflags! {
    #[derive(Debug, Clone, Copy, PartialEq, Eq)]
    pub struct Perms: u8 {
        const READ = 1;
        const WRITE = 2;
        const EXEC = 4;
        const RW = Self::READ.bits() | Self::WRITE.bits();
    }
}

pub fn report() -> String {
    let mut lines = Vec::new();
    let rw = Perms::READ | Perms::WRITE;
    lines.push(format!("{rw:?}"));
    lines.push(format!("{}", rw.bits()));
    lines.push(format!("{}", rw.contains(Perms::READ)));
    lines.push(format!("{}", rw.contains(Perms::EXEC)));
    lines.push(format!("{:?}", rw - Perms::READ));
    lines.push(format!("{:?}", !rw));
    lines.push(format!("{:?}", Perms::all()));
    lines.push(format!("{:?}", Perms::empty()));
    lines.push(format!("{:?}", Perms::from_bits(9)));
    lines.push(format!("{:?}", Perms::from_bits_truncate(9)));
    lines.push(format!("{:?}", Perms::from_bits_retain(9)));
    lines.push(format!("{:?}", Perms::from_name("EXEC")));
    lines.push(format!("{:?}", Perms::from_name("NOPE")));
    let names: Vec<_> = (Perms::READ | Perms::EXEC)
        .iter_names()
        .map(|(n, f)| format!("{n}={}", f.bits()))
        .collect();
    lines.push(names.join(","));
    let each: Vec<_> = Perms::all().iter().map(|f| f.bits()).collect();
    lines.push(format!("{each:?}"));
    let mut text = String::new();
    bitflags::parser::to_writer(&(Perms::READ | Perms::EXEC), &mut text).unwrap();
    lines.push(text);
    for input in ["READ | WRITE", "EXEC|0x8", "", "READ | NOPE", "0xzz", "READ ||"] {
        match bitflags::parser::from_str::<Perms>(input) {
            Ok(p) => lines.push(format!("ok {p:?}")),
            Err(e) => lines.push(format!("err {e}")),
        }
    }
    let mut p = Perms::empty();
    p.insert(Perms::EXEC);
    p.toggle(Perms::READ);
    p.set(Perms::WRITE, true);
    p.remove(Perms::READ);
    lines.push(format!("{p:?} {}", p.is_all()));
    lines.join("\n")
}
