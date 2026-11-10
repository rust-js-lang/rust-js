//@ native: wasm32
// `p.byte_offset_from(q)`: how many bytes `p` is past `q` in one
// allocation, their addresses' difference, `$addr(p, size) - $addr(q,
// size)`: of items of one array, either way; of a pointer of another type
// at any byte of the other's, 3 bytes into a word; of a word stepped 2
// bytes; of a slice's range in generic code; and of the fields of one
// `#[repr(C)]` struct, its start and each other's, as wasm32 lays them out,
// of a field and an item of another field's array, and of a whole array
// and its item, each address asked by its own pointee's size; and of text's bytes, a copy's, within it
// (#1256).
#[repr(C)]
struct Header {
    tag: u32,
    len: u16,
    flags: u16,
}

#[repr(C)]
struct Row {
    id: u16,
    cells: [u32; 3],
}

fn bytes_of<T>(items: &[T]) -> isize {
    let r = items.as_ptr_range();
    unsafe { r.end.byte_offset_from(r.start) }
}

fn main() {
    let a = [1u32, 2, 3, 4];
    let p = a.as_ptr();
    println!("{} {}", unsafe { p.add(2).byte_offset_from(p) }, unsafe {
        p.byte_offset_from(p.add(3))
    });
    let q = unsafe { p.cast::<u8>().add(3) };
    println!("{} {}", unsafe { p.add(1).byte_offset_from(q) }, unsafe {
        q.byte_offset_from(p)
    });
    let w = unsafe { p.cast::<u8>().add(2).cast::<u32>() };
    println!("{}", unsafe { w.byte_offset_from(p.add(3)) });
    let names = vec![String::from("a"), String::from("b")];
    let empty: Vec<u16> = Vec::new();
    println!("{} {} {}", bytes_of(&a), bytes_of(&names), bytes_of(&empty));
    let header = Header {
        tag: 7,
        len: 2,
        flags: 1,
    };
    let at = &raw const header;
    let len = &raw const header.len;
    let flags = &raw const header.flags;
    println!(
        "{} {} {}",
        unsafe { flags.byte_offset_from(at) },
        unsafe { flags.byte_offset_from(len) },
        unsafe { at.byte_offset_from(flags) }
    );
    println!("{}", header.tag + u32::from(header.len) + u32::from(header.flags));
    let row = Row {
        id: 1,
        cells: [4, 5, 6],
    };
    let id = &raw const row.id;
    let cell = &raw const row.cells[2];
    println!("{}", unsafe { id.byte_offset_from(cell) });
    println!("{} {:?}", row.id, row.cells);
    let shorts = [9u16, 8, 7];
    let whole = &raw const shorts;
    println!("{}", unsafe { whole.byte_offset_from(shorts.as_ptr().add(2)) });
    let text = String::from("hello");
    let bytes = text.as_bytes().as_ptr();
    println!("{}", unsafe { bytes.add(4).byte_offset_from(bytes) });
}
