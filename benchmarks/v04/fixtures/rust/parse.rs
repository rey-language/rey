use std::env;

fn main() {
    let args: Vec<String> = env::args().collect();
    if args.len() != 10 { std::process::exit(2); }
    let size = args[1].parse::<i64>().unwrap_or(-1);
    if !(0..=5_000_000).contains(&size) { std::process::exit(2); }
    let mut checksum = 0_i64;
    for i in 0..size {
        let value = args[((i % 8) + 2) as usize].parse::<i64>().unwrap_or_else(|_| std::process::exit(2));
        checksum += value;
    }
    println!("checksum={checksum} processed={size}");
}
