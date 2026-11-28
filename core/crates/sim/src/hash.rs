//! 결정론 상태 해시 (이슈 #33).
//!
//! 같은 맵·명령·시드면 OS와 무관하게 같은 값이 나와야 한다. 표준 라이브러리의 기본 해셔는
//! Rust 버전 사이에 값이 바뀔 수 있어 쓰지 않고, 고정된 FNV-1a 64비트를 정해진 순서로 쓴다.
//! 실수는 비트 그대로 넣는다 — 계산이 플랫폼마다 1비트라도 다르면 해시가 달라져 바로 드러난다.

use crate::traffic::VehicleState;
use crate::Simulation;

/// FNV-1a 64비트.
#[derive(Debug, Clone)]
pub struct StableHasher(u64);

impl Default for StableHasher {
    fn default() -> Self {
        Self(0xcbf2_9ce4_8422_2325)
    }
}

impl StableHasher {
    pub fn bytes(&mut self, b: &[u8]) {
        for &x in b {
            self.0 ^= x as u64;
            self.0 = self.0.wrapping_mul(0x0000_0100_0000_01b3);
        }
    }
    pub fn u64(&mut self, v: u64) {
        self.bytes(&v.to_le_bytes());
    }
    pub fn f64(&mut self, v: f64) {
        self.u64(v.to_bits());
    }
    pub fn str(&mut self, s: &str) {
        self.u64(s.len() as u64);
        self.bytes(s.as_bytes());
    }
    pub fn opt(&mut self, v: Option<u64>) {
        match v {
            Some(x) => {
                self.u64(1);
                self.u64(x);
            }
            None => self.u64(0),
        }
    }
    pub fn finish(&self) -> u64 {
        self.0
    }
}

fn state_code(s: VehicleState) -> u64 {
    match s {
        VehicleState::Scheduled => 0,
        VehicleState::WaitingToEnter => 1,
        VehicleState::Crossing => 2,
        VehicleState::Queued => 3,
        VehicleState::WaitingToAdvance => 4,
        VehicleState::Arrived => 5,
        VehicleState::Cancelled => 6,
    }
}

impl Simulation {
    /// 시뮬레이션 상태 해시: 시각, 도로 그래프, 구역, 차량, 명령 결과, 달력.
    /// 테스트·리플레이·플랫폼 간 검증용. 화면용 값(차량 표시 위치 등)은 넣지 않는다.
    pub fn state_hash(&self) -> u64 {
        let mut h = StableHasher::default();
        h.str("dongne-state-v1");
        h.u64(self.tick());

        let g = &self.world.road_graph;
        h.u64(g.nodes.len() as u64);
        for n in &g.nodes {
            h.f64(n.x);
            h.f64(n.y);
            h.u64(n.boundary as u64);
        }
        h.u64(g.edges.len() as u64);
        for e in &g.edges {
            h.u64(e.from as u64);
            h.u64(e.to as u64);
            h.u64(e.osm_id as u64);
            h.str(&e.highway);
            h.f64(e.length_m);
            h.u64(e.lanes_forward as u64);
            h.u64(e.lanes_backward as u64);
            h.u64(e.removed as u64);
        }
        h.u64(g.lanes.len() as u64);
        for l in &g.lanes {
            h.u64(l.capacity as u64);
        }

        for (r, c, z) in self.world.zones.zoned() {
            h.u64(r as u64);
            h.u64(c as u64);
            h.str(&format!("{z:?}"));
        }

        h.u64(self.traffic.now_ms());
        h.u64(self.traffic.vehicles.len() as u64);
        for v in &self.traffic.vehicles {
            h.u64(v.origin as u64);
            h.u64(v.dest as u64);
            h.u64(v.depart_ms);
            h.opt(v.arrive_ms);
            h.u64(state_code(v.state));
            h.opt(v.lane.map(u64::from));
            h.u64(v.route.len() as u64);
            for &a in &v.route {
                h.u64(a as u64);
            }
            h.u64(v.trace.len() as u64);
            for &l in &v.trace {
                h.u64(l as u64);
            }
        }

        h.str(&serde_json::to_string(&self.outcomes).expect("직렬화 가능"));
        for e in &self.calendar.log {
            h.str(&format!("{e:?}"));
        }
        for s in &self.calendar.seasons {
            h.u64(s.season as u64);
            h.u64(s.total_trips);
            h.f64(s.mean_commute_s);
        }
        h.finish()
    }

    /// `state_hash`를 16자리 16진수로.
    pub fn state_hash_hex(&self) -> String {
        format!("{:016x}", self.state_hash())
    }
}

#[cfg(test)]
mod tests {
    use super::StableHasher;

    #[test]
    fn fnv1a_known_values() {
        // FNV-1a 64 표준 시험값
        let mut h = StableHasher::default();
        h.bytes(b"");
        assert_eq!(h.finish(), 0xcbf29ce484222325);
        let mut h = StableHasher::default();
        h.bytes(b"a");
        assert_eq!(h.finish(), 0xaf63dc4c8601ec8c);
        let mut h = StableHasher::default();
        h.bytes(b"foobar");
        assert_eq!(h.finish(), 0x85944171f73967e8);
    }
}
