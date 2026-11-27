//! 샘플 맵 maps/pyeongtaek-station에서 차량 100대가 모두 도착하고, 같은 시드면 결과가 같은지 확인한다.

use std::path::PathBuf;

use dongne_sim_core::{Simulation, TICK_MS};
use dongne_world::load_map;

const SEED: u64 = 42;
const VEHICLES: usize = 100;
/// 5분 동안 고르게 출발 (가정)
const DEPART_WINDOW_MS: u64 = 300_000;
/// 안전장치: 시뮬레이션 3시간 안에 끝나야 함
const LIMIT_MS: u64 = 3 * 3_600_000;

fn sim(seed: u64) -> Simulation {
    let dir = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../../maps/pyeongtaek-station");
    let mut sim = Simulation::new(load_map(&dir).unwrap());
    assert_eq!(
        sim.add_random_trips(VEHICLES, seed, DEPART_WINDOW_MS),
        VEHICLES
    );
    sim
}

fn run(seed: u64) -> Simulation {
    let mut s = sim(seed);
    while s.traffic.stats().arrived < VEHICLES && s.tick() * TICK_MS < LIMIT_MS {
        s.step(600); // 1분씩
    }
    s
}

#[test]
fn hundred_vehicles_arrive_deterministically() {
    let a = run(SEED);
    let s = a.traffic.stats();
    println!("{s:#?}");
    assert_eq!(s.arrived, VEHICLES, "모든 차량이 도착해야 함");
    assert!(s.mean_travel_s > 0.0);

    let b = run(SEED);
    assert_eq!(s, b.traffic.stats());
    assert_eq!(a.traffic.vehicles, b.traffic.vehicles);

    let od = |x: &Simulation| {
        x.traffic
            .vehicles
            .iter()
            .map(|v| (v.origin, v.dest, v.depart_ms))
            .collect::<Vec<_>>()
    };
    assert_ne!(od(&a), od(&sim(SEED + 1)), "다른 시드면 다른 출발·도착");
}
