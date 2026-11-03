// A loop's `match` whose arm ends `if c { a } else { b } continue;` keeps
// the `if`/`else` as written, with one `continue` after it, as React's
// ReactFlightClient processBinaryChunk writes its ROW_ID, ROW_TAG and
// ROW_LENGTH cases (rd703): of an `if`/`else`, and of an `if`/`else if`/
// `else` whose last branch ends with a comment.
const ROW_ID: u32 = 0;
const ROW_TAG: u32 = 1;
const ROW_LENGTH: u32 = 2;
const ROW_CHUNK: u32 = 3;

pub fn parse(chunk: &[u8]) -> (u32, u32, u32, u32, usize) {
    let mut i = 0;
    let mut row_state = ROW_ID;
    let mut row_id = 0u32;
    let mut row_tag = 0u32;
    let mut row_length = 0u32;
    let mut seen = 0usize;
    while i < chunk.len() {
        let mut last_idx = 0;
        match row_state {
            ROW_ID => {
                let byte = chunk[i] as u32;
                i += 1;
                if byte == 58 {
                    // Finished the rowID.
                    row_state = ROW_TAG;
                } else {
                    row_id = (row_id << 4) | (byte & 15);
                }
                continue;
            }
            ROW_TAG => {
                let resolved_row_tag = chunk[i] as u32;
                if resolved_row_tag == 84 || resolved_row_tag == 86 {
                    row_tag = resolved_row_tag;
                    row_state = ROW_LENGTH;
                    i += 1;
                } else if resolved_row_tag > 64 && resolved_row_tag < 91 {
                    row_tag = resolved_row_tag;
                    row_state = ROW_CHUNK;
                    i += 1;
                } else {
                    row_tag = 0;
                    row_state = ROW_CHUNK;
                    // This was an unknown tag so it was probably part of the data.
                }
                continue;
            }
            ROW_LENGTH => {
                let byte = chunk[i] as u32;
                i += 1;
                if byte == 44 {
                    row_state = ROW_CHUNK;
                } else {
                    row_length = (row_length << 4) | (byte & 15);
                }
                continue;
            }
            ROW_CHUNK => {
                last_idx = i + 1;
                row_state = ROW_ID;
            }
            _ => {}
        }
        seen += last_idx;
        i += 1;
    }
    (row_state, row_id, row_tag, row_length, seen)
}

fn main() {
    println!("{:?}", parse(b"12:T3,456:x9"));
    println!("{:?}", parse(b"7:Bq1:%z"));
    println!("{:?}", parse(b"a:V12,b:"));
}
