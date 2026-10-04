use std::time::Instant;

use memobi_poarm::{work, Config};

fn main() {
    let config = Config::default();
    let seed = b"memobi-poarm-lab";
    let samples = 100u64;

    let started = Instant::now();

    let mut accumulator = [0u8; 32];
    for nonce in 0..samples {
        let result = work(seed, nonce, config);
        for (a, b) in accumulator.iter_mut().zip(result) {
            *a ^= b;
        }
    }

    let elapsed = started.elapsed();
    let seconds = elapsed.as_secs_f64();
    let rate = samples as f64 / seconds.max(f64::MIN_POSITIVE);

    println!("PoARM laboratory benchmark");
    println!("version: {}", memobi_poarm::VERSION);
    println!("memory: {} KiB", config.memory_kib);
    println!("rounds: {}", config.rounds);
    println!("samples: {}", samples);
    println!("elapsed: {:.3} s", seconds);
    println!("rate: {:.2} work/s", rate);
    println!("checksum: {:02x?}", accumulator);
}
