//! 시뮬레이션 틱 루프. 고정 틱, 결정론적.

use dongne_world::World;

/// 시뮬레이션 상태.
#[derive(Debug)]
pub struct Simulation {
    pub world: World,
    tick: u64,
}

impl Simulation {
    pub fn new(world: World) -> Self {
        Self { world, tick: 0 }
    }

    /// 지금까지 진행한 틱 수.
    pub fn tick(&self) -> u64 {
        self.tick
    }

    /// n틱 진행한다. 0단계에서는 틱 수만 센다.
    pub fn step(&mut self, n_ticks: u64) {
        self.tick += n_ticks;
    }
}
