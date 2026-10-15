// A static that refers to another static's number, `&A.count`, is that
// number: a reference is its value (ADR 0023), and the other can't change,
// so no program tells them apart (ADR 0096).
struct Settings {
    count: usize,
    ratio: f64,
}

static SETTINGS: Settings = Settings { count: 3, ratio: 0.5 };
static COUNT: &usize = &SETTINGS.count;
static RATIO: &f64 = &SETTINGS.ratio;
static LIMITS: [u32; 2] = [10, 20];
static FIRST: &u32 = &LIMITS[0];
static LAST: u32 = LIMITS[1];

fn main() {
    println!("{} {} {}", *COUNT, *RATIO, *COUNT == SETTINGS.count);
    println!("{} {} {}", *FIRST, LAST, *FIRST + LAST);
}
