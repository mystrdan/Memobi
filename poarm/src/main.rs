use std::time::Instant;

use memobi_core::{BlockHeight, Hash32, chain::ChainState, genesis::{GenesisConfig, build_genesis}, reward::RewardConfig, block_builder::BlockTemplate};
use memobi_poarm::{Config, work, work_candidate_b, work_candidate_c, miner::search_candidate_c};

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

    println!();
    println!("Devnet mining smoke test");
    let mut chain = ChainState::default();
    let genesis = build_genesis(GenesisConfig::provisional());
    chain.apply_block(&genesis).expect("apply genesis");
    println!("  genesis_height: {}", chain.height.unwrap());

    let miner_pubkey = memobi_core::crypto::SecretKey::from_bytes(&[42u8; 32]).public_key();
    let template = BlockTemplate {
        version: 1,
        previous_block: chain.tip.unwrap(),
        height: BlockHeight(chain.height.unwrap() + 1),
        timestamp: 1,
        target: u64::MAX,
        poarm_version: memobi_poarm::VERSION,
        poarm_nonce: 0,
    };
    let epoch = template.height.0;
    let mut block = template
        .build_mining_block(Vec::new(), miner_pubkey.to_vec(), RewardConfig::provisional())
        .expect("template");

    let seed_hash = block.header.poarm_seed(epoch).expect("seed");
    let mining = search_candidate_c(
        seed_hash.as_bytes(),
        epoch,
        Config { memory_kib: 1, rounds: 1 },
        block.header.target,
        0,
        10_000,
    )
    .expect("easy devnet target should be found");
    block.header.poarm_nonce = mining.nonce;
    // Independent verification before acceptance (never trust miner claim).
    assert!(memobi_poarm::miner::verify_candidate_c(
        seed_hash.as_bytes(),
        epoch,
        Config { memory_kib: 1, rounds: 1 },
        block.header.target,
        mining.nonce,
        &mining.proof,
    ));

    let block_id = chain
        .apply_validated_block(&block, Hash32(mining.proof))
        .expect("validated block");
    println!("  mined_height: {}", chain.height.unwrap());
    println!("  nonce: {}", mining.nonce);
    println!("  attempts: {}", mining.attempts);
    println!("  block_id: {:02x?}", block_id.as_bytes());
    println!("  status: end-to-end block accepted");
}
