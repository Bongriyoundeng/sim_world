//! 시뮬레이션 틱 루프. 고정 틱, 결정론적.

use std::cmp::Reverse;

use dongne_world::World;

pub mod traffic;
pub use traffic::{Rng, Traffic, TrafficStats, Vehicle, VehicleState};

/// 틱 하나의 시뮬레이션 시간 (ms, 가정). 교통 이벤트는 틱 경계까지 처리된다.
pub const TICK_MS: u64 = 100;

/// 시뮬레이션 상태.
#[derive(Debug)]
pub struct Simulation {
    pub world: World,
    pub traffic: Traffic,
    tick: u64,
}

impl Simulation {
    pub fn new(world: World) -> Self {
        let traffic = Traffic::new(&world.road_graph);
        Self {
            world,
            traffic,
            tick: 0,
        }
    }

    /// 지금까지 진행한 틱 수.
    pub fn tick(&self) -> u64 {
        self.tick
    }

    /// n틱 진행한다. 그 시각까지의 교통 이벤트를 처리한다.
    pub fn step(&mut self, n_ticks: u64) {
        self.tick += n_ticks;
        self.traffic.run_until(self.tick * TICK_MS);
    }

    /// 가장 큰 강연결 요소 안에서 무작위 출발·도착 노드로 차량 `n`대를 예약한다.
    /// 출발 시각은 `[0, depart_window_ms)`에서 고르게 뽑는다. 같은 시드면 같은 차량이 나온다.
    /// 예약한 차량 수를 돌려준다 (경로를 못 찾는 쌍은 다시 뽑고, 시도 횟수를 넘으면 그만둔다).
    pub fn add_random_trips(&mut self, n: usize, seed: u64, depart_window_ms: u64) -> usize {
        let comps = self.world.road_graph.strongly_connected_components();
        let Some(nodes) = comps.iter().max_by_key(|c| (c.len(), Reverse(c[0]))) else {
            return 0;
        };
        if nodes.len() < 2 {
            return 0;
        }
        let mut rng = Rng::new(seed);
        let mut added = 0;
        let mut attempts = 0;
        while added < n && attempts < n * 20 {
            attempts += 1;
            let o = nodes[rng.below(nodes.len() as u64) as usize];
            let d = nodes[rng.below(nodes.len() as u64) as usize];
            let t = rng.below(depart_window_ms.max(1));
            if o != d && self.traffic.add_trip(o, d, t).is_some() {
                added += 1;
            }
        }
        added
    }
}
