use uuid::Uuid;

pub fn report() -> String {
    let mut lines = Vec::new();
    for text in [
        "67e55044-10b1-426f-9247-bb680e5fe0c8",
        "67E5504410B1426F9247BB680E5FE0C8",
        "urn:uuid:67e55044-10b1-426f-9247-bb680e5fe0c8",
        "{67e55044-10b1-426f-9247-bb680e5fe0c8}",
        "00000000-0000-0000-0000-000000000000",
        "not a uuid",
        "67e55044-10b1-426f-9247-bb680e5fe0c",
        "67e55044-10b1-426f-9247-bb680e5fe0cg",
    ] {
        match Uuid::parse_str(text) {
            Ok(id) => lines.push(format!(
                "{text:?}: {id} {} {} {} v{} nil {} {:?}",
                id.hyphenated(),
                id.simple(),
                id.urn(),
                id.get_version_num(),
                id.is_nil(),
                id.get_variant(),
            )),
            Err(error) => lines.push(format!("{text:?}: {error}")),
        }
    }
    let bytes = [
        0xa1, 0xa2, 0xa3, 0xa4, 0xb1, 0xb2, 0xc1, 0xc2, 0xd1, 0xd2, 0xd3, 0xd4, 0xd5, 0xd6, 0xd7, 0xd8,
    ];
    let id = Uuid::from_bytes(bytes);
    lines.push(format!("{id} {:?} {:?}", id.as_bytes(), id.as_fields()));
    lines.push(format!("{} {}", Uuid::nil(), Uuid::max()));
    lines.push(format!(
        "{}",
        Uuid::from_u128(0x1234_5678_9abc_def0_1234_5678_9abc_def0)
    ));
    lines.push(format!("{:x}", Uuid::from_u128(1).as_u128()));
    let mut ids = vec![Uuid::from_u128(3), Uuid::nil(), Uuid::from_u128(2)];
    ids.sort();
    lines.push(format!("{ids:?}"));
    lines.join("\n")
}
