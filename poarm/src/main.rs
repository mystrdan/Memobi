use std::time::Instant;

use memobi_poarm::{work, work_candidate_b, work_candidate_c, Config};

fn benchmark<F>(name: &str, samples: u64, mut f: F)
where
    F: FnMut(u64) -> [u8; 32],
{
    let started = Instant::now();
    let mut accumulator = [0u8; 32];

    for nonce in 0..samples {
        let result = f(nonce);
        for (a, b) in accumulator.iter_mut().zip(result) {
            *a ^= b;
        }
    }

    let elapsed = started.elapsed();
    let seconds = elapsed.as_secs_f64();
    let rate = samples as f64 / seconds.max(f64::MIN_POSITIVE);

    println!("{name}");
    println!("  elapsed_seconds: {:.6}", seconds);
    println!("  work_per_second: {:.3}", rate);
    println!("  checksum: {:02x?}", accumulator);
}

fn main() {
    let config = Config::default();
    let seed = b"memobi-poarm-lab";
    let samples = 100u64;
    let epoch = 7u64;

    println!("PoARM candidate comparison");
    println!("version: {}", memobi_poarm::VERSION);
    println!("memory_kib: {}", config.memory_kib);
    println!("rounds: {}", config.rounds);
    println!("samples: {}", samples);
    println!("epoch_for_candidate_c: {}", epoch);
    println!();

    benchmark("candidate_a_baseline", samples, |nonce| {
        work(seed, nonce, config)
    });
    benchmark("candidate_b_dependency_chain", samples, |nonce| {
        work_candidate_b(seed, nonce, config)
    });
    benchmark("candidate_c_epoch_parameterized", samples, |nonce| {
        work_candidate_c(seed, nonce, epoch, config)
    });
}
