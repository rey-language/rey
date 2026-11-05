use std::env;

fn main() {
    let args: Vec<String> = env::args().collect();
    if args.len() != 2 { std::process::exit(2); }
    let size: i64 = match args[1].parse() {
        Ok(value) if (0..=10_000_000).contains(&value) => value,
        _ => std::process::exit(2),
    };
    let mut state = 1_i64;
    let mut checksum = 0_i64;
    for _ in 0..size {
        state = (state * 48271) % 2147483647;
        checksum += state % 1024;
    }
    println!("checksum={checksum} processed={size}");
}
