use std::env;

fn main() {
    let args: Vec<String> = env::args().collect();
    if args.len() != 2 { std::process::exit(2); }
    let size = args[1].parse::<i64>().unwrap_or(-1);
    if !(0..=10_000_000).contains(&size) { std::process::exit(2); }
    let mut state = 1_i64;
    let mut checksum = 0_i64;
    for _ in 0..size {
        state = (state * 48271) % 2147483647;
        if state % 7 == 0 { checksum = (checksum * 31 + state) % 2147483647; }
    }
    println!("checksum={checksum} processed={size}");
}
