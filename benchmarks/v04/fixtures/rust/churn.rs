use std::env;

fn main() {
    let args: Vec<String> = env::args().collect();
    if args.len() != 2 { std::process::exit(2); }
    let rounds: i64 = match args[1].parse() {
        Ok(value) if (0..=4096).contains(&value) => value,
        _ => std::process::exit(2),
    };
    let mut checksum = 0_i64;
    for round in 0..rounds {
        let mut values = Vec::<i64>::new();
        for index in 0..32_i64 {
            values.push((round * 17 + index * 13) % 1009);
        }
        for value in &values { checksum += *value; }
    }
    println!("checksum={checksum} processed={}", rounds * 32);
}
