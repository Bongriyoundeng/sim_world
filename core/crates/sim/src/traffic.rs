//! 이벤트 기반 차선 대기열 교통 모델 (ADR 0004).
//!
//! - 시간은 정수 밀리초. 이벤트는 (시각, 순번) 순서로 처리해 결정론을 지킨다.
//! - 차량은 차선 주행(Crossing) → 차선 끝 대기열(Queued) → 맨 앞에서 교차로 통과 대기
//!   (WaitingToAdvance) → 다음 차선 순서로 움직인다. 가감속과 추월은 없다.
//! - 차선마다 대기열 용량(도로 그래프의 길이 ÷ 차량 간격)이 있고, 가득 차면 들어가지 못한다.
//! - 교차로 (이슈 #34): 들어오는 차선마다 포화 차두시간(2초)에 한 대씩 떠난다. 진입로가 3개
//!   이상인 교차로는 2현시 고정 신호(주도로 축·교차 축)로, 녹색일 때만 떠난다. 진입로 2개 이하는
//!   신호 없음. 교차로를 지나는 데 `INTERSECTION_MS`가 걸린다. 회전 충돌은 따로 계산하지 않는다.
//!
//! 근거와 가정: docs/design/research-log.md#traffic-queue, #signals

use std::cmp::Reverse;
use std::collections::{BinaryHeap, VecDeque};

use dongne_world::RoadGraph;

/// 교차로를 지나는 시간 (ms, 가정). 경로 비용에도 더한다.
pub const INTERSECTION_MS: u64 = 2_000;
/// 같은 차선에서 연달아 떠나는 간격 = 포화 차두시간 (ms). FHWA Signal Timing Manual 3.4.3의
/// 실무 근사 "차로당 1,800대/시 → 차량당 녹색 2초". research-log.md#signals
pub const SATURATION_HEADWAY_MS: u64 = 2_000;
/// 신호 주기 (ms). FHWA Signal Timing Manual 표 3-4: 양쪽 도로 비보호 좌회전 계획 주기 60초.
pub const SIGNAL_CYCLE_MS: u64 = 60_000;
/// 현시당 손실 시간 (ms, 황색 + 전적색 등). HCM 기본값 4초 (FHWA STM 3.3.3).
pub const LOST_TIME_PER_PHASE_MS: u64 = 4_000;
/// 최소 녹색 시간 (ms, 가정).
pub const MIN_GREEN_MS: u64 = 10_000;
/// 차선 맨 앞에서 다음 차선이 가득 차 이만큼 기다리면 다른 길을 찾는다 (ms, 가정).
/// 대기열이 서로 막아 아무도 못 움직이는 교착(gridlock)을 풀기 위한 운전자 행동.
pub const REROUTE_WAIT_MS: u64 = 60_000;
/// 다른 길도 없이 이만큼 기다리면 다음 차선 용량을 넘겨서라도 들어간다 (ms, 가정).
/// 대기열 모델에서 서로 막힌 순환이 영원히 풀리지 않는 것을 막는 마지막 수단이며, 몇 번 일어났는지
/// `TrafficStats::jam_relief`로 센다.
pub const JAM_RELIEF_WAIT_MS: u64 = 300_000;

/// 결정론적 난수 (SplitMix64). 같은 시드면 같은 수열.
#[derive(Debug, Clone)]
pub struct Rng(u64);

impl Rng {
    pub fn new(seed: u64) -> Self {
        Self(seed)
    }

    pub fn next_u64(&mut self) -> u64 {
        self.0 = self.0.wrapping_add(0x9E37_79B9_7F4A_7C15);
        let mut z = self.0;
        z = (z ^ (z >> 30)).wrapping_mul(0xBF58_476D_1CE4_E5B9);
        z = (z ^ (z >> 27)).wrapping_mul(0x94D0_49BB_1331_11EB);
        z ^ (z >> 31)
    }

    /// 0..n 범위의 수 (n > 0).
    pub fn below(&mut self, n: u64) -> u64 {
        self.next_u64() % n
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
enum Event {
    /// 차량 출발 시각이 됨
    Depart(u32),
    /// 차량이 현재 차선 끝에 닿음
    ReachEnd(u32),
    /// 교차로를 다시 볼 시각 (신호가 바뀌거나 차선 차두시간이 끝남)
    Wake(u32),
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum VehicleState {
    /// 출발 전
    Scheduled,
    /// 출발했지만 첫 차선에 자리가 나기를 기다림
    WaitingToEnter,
    /// 차선 위를 달리는 중
    Crossing,
    /// 차선 끝에 닿았지만 앞차가 남아 있음
    Queued,
    /// 차선 맨 앞에서 교차로 통과를 기다림
    WaitingToAdvance,
    Arrived,
    /// 도로가 바뀌어 목적지로 갈 길이 없어져 통행을 포기함 (도로에서 빠짐)
    Cancelled,
}

#[derive(Debug, Clone, PartialEq)]
pub struct Vehicle {
    pub id: u32,
    pub origin: u32,
    pub dest: u32,
    pub depart_ms: u64,
    pub arrive_ms: Option<u64>,
    pub state: VehicleState,
    /// 방향 엣지(arc) 경로. arc = 엣지 × 2 + (역방향이면 1)
    pub route: Vec<u32>,
    /// 지금 있는 arc의 경로 안 위치 (출발 전·대기 중이면 None)
    pub step: Option<usize>,
    pub lane: Option<u32>,
    /// 지나간 차선 (클라이언트 재생·검증용)
    pub trace: Vec<u32>,
    at_end: bool,
    /// 지금 차선에서 주행을 시작한 시각 (교차로 통과 후)
    lane_start_ms: u64,
    lane_travel_ms: u64,
    /// 교차로 통과(또는 출발)를 기다리기 시작한 시각
    wait_since_ms: u64,
    /// 다음에 다른 길을 찾아볼 시각
    reroute_at_ms: u64,
}

#[derive(Debug, Clone)]
struct ArcInfo {
    tail: u32,
    head: u32,
    travel_ms: u64,
    lanes: Vec<u32>,
    /// 명령으로 없앤 도로
    removed: bool,
    /// head 노드로 들어가는 진행 방향 (단위 벡터, 신호 현시 나누기용)
    dir_in: (f64, f64),
    /// tertiary 이상 간선 (신호 위치 규칙)
    major: bool,
}

/// 신호 위치 규칙에서 간선으로 보는 도로 유형 (tertiary 이상과 그 연결로).
pub fn is_major_road(highway: &str) -> bool {
    matches!(
        highway.trim_end_matches("_link"),
        "motorway" | "trunk" | "primary" | "secondary" | "tertiary"
    )
}

#[derive(Debug, Clone)]
struct LaneState {
    arc: u32,
    capacity: u32,
    blocked: bool,
    queue: VecDeque<u32>,
    /// 이 차선 맨 앞 차량이 교차로로 떠날 수 있는 가장 이른 시각 (포화 차두시간)
    free_at: u64,
}

/// 2현시 고정 신호. 주기 = 녹색A + 손실 + 녹색B + 손실.
#[derive(Debug, Clone, PartialEq)]
pub struct Signal {
    /// (들어오는 arc, 현시 0 또는 1)
    pub phase_of: Vec<(u32, u8)>,
    pub green_ms: [u64; 2],
    pub offset_ms: u64,
}

impl Signal {
    fn phase(&self, arc: u32) -> Option<u8> {
        self.phase_of.iter().find(|x| x.0 == arc).map(|x| x.1)
    }

    fn start(&self, p: u8) -> u64 {
        if p == 0 {
            0
        } else {
            self.green_ms[0] + LOST_TIME_PER_PHASE_MS
        }
    }

    /// `arc`에서 들어온 차량이 `t`에 떠날 수 있나 (현시에 없는 arc는 항상 가능).
    pub fn is_green(&self, arc: u32, t: u64) -> bool {
        let Some(p) = self.phase(arc) else {
            return true;
        };
        let tau = (t + self.offset_ms) % SIGNAL_CYCLE_MS;
        let s = self.start(p);
        tau >= s && tau < s + self.green_ms[p as usize]
    }

    /// `t` 이후 `arc`의 다음 녹색 시작 시각.
    pub fn next_green(&self, arc: u32, t: u64) -> u64 {
        let Some(p) = self.phase(arc) else {
            return t;
        };
        let tau = (t + self.offset_ms) % SIGNAL_CYCLE_MS;
        let s = self.start(p);
        t + (s + SIGNAL_CYCLE_MS - tau) % SIGNAL_CYCLE_MS
    }
}

#[derive(Debug, Clone, Default)]
struct NodeState {
    /// 통과를 기다리는 차량 (요청 순서)
    waiting: Vec<u32>,
    /// 이 노드에서 출발하는 차량이 첫 차선에 들어갈 수 있는 가장 이른 시각
    spawn_free_at: u64,
    /// 예약해 둔 가장 이른 Wake 시각
    wake_at: Option<u64>,
    signal: Option<Signal>,
}

#[derive(Debug, Clone, PartialEq)]
pub struct TrafficStats {
    pub vehicles: usize,
    pub arrived: usize,
    pub mean_travel_s: f64,
    pub max_travel_s: f64,
    pub now_s: f64,
    /// 교착 해소로 차선 용량을 넘겨 들어간 횟수 (0이 정상)
    pub jam_relief: u64,
    /// 도로가 바뀌어 갈 길이 없어져 포기한 통행 수
    pub cancelled: u64,
}

#[derive(Debug, Clone)]
pub struct Traffic {
    arcs: Vec<Option<ArcInfo>>,
    node_out: Vec<Vec<u32>>,
    lanes: Vec<LaneState>,
    nodes: Vec<NodeState>,
    pub vehicles: Vec<Vehicle>,
    events: BinaryHeap<Reverse<(u64, u64, Event)>>,
    seq: u64,
    now_ms: u64,
    jam_relief: u64,
    cancelled: u64,
}

fn reverse(arc: u32) -> u32 {
    arc ^ 1
}

impl Traffic {
    pub fn new(g: &RoadGraph) -> Self {
        let mut t = Self {
            arcs: Vec::new(),
            node_out: Vec::new(),
            lanes: Vec::new(),
            nodes: Vec::new(),
            vehicles: Vec::new(),
            events: BinaryHeap::new(),
            seq: 0,
            now_ms: 0,
            jam_relief: 0,
            cancelled: 0,
        };
        t.sync(g);
        t
    }

    /// 도로 그래프에 새로 생긴 노드·엣지·차선을 반영한다 (기존 것은 그대로).
    /// 그래프의 노드·엣지·차선은 뒤에 붙기만 하므로 ID가 그대로 유지된다.
    pub fn sync(&mut self, g: &RoadGraph) {
        self.nodes.resize(g.nodes.len(), NodeState::default());
        self.node_out.resize(g.nodes.len(), Vec::new());
        let first_new_edge = self.arcs.len() / 2;
        self.arcs.resize(g.edges.len() * 2, None);
        for e in &g.edges[first_new_edge..] {
            let speed_ms = e.free_speed_kmh / 3.6 / 1000.0; // m per ms
            let travel_ms = (e.length_m / speed_ms).ceil().max(1.0) as u64;
            for (forward, n) in [(true, e.lanes_forward), (false, e.lanes_backward)] {
                if n == 0 {
                    continue;
                }
                let (tail, head) = if forward {
                    (e.from, e.to)
                } else {
                    (e.to, e.from)
                };
                let arc = e.id * 2 + u32::from(!forward);
                let k = e.points.len();
                let (a, b) = if forward {
                    (e.points[k - 2], e.points[k - 1])
                } else {
                    (e.points[1], e.points[0])
                };
                let len = dongne_world::road_graph::norm(b.0 - a.0, b.1 - a.1).max(1e-9);
                self.arcs[arc as usize] = Some(ArcInfo {
                    tail,
                    head,
                    travel_ms,
                    lanes: Vec::new(),
                    removed: e.removed,
                    dir_in: ((b.0 - a.0) / len, (b.1 - a.1) / len),
                    major: is_major_road(&e.highway),
                });
                let out = &mut self.node_out[tail as usize];
                out.push(arc);
                out.sort_unstable();
            }
        }
        let first_new_lane = self.lanes.len();
        for l in &g.lanes[first_new_lane..] {
            let arc = l.edge * 2 + u32::from(!l.forward);
            if let Some(a) = self.arcs[arc as usize].as_mut() {
                a.lanes.push(l.id);
            }
            self.lanes.push(LaneState {
                arc,
                capacity: l.capacity,
                blocked: false,
                queue: VecDeque::new(),
                free_at: 0,
            });
        }
        self.rebuild_signals();
    }

    /// 교차로마다 신호를 다시 정한다 (도로가 바뀔 때).
    ///
    /// 신호 위치 규칙 (가정 — 평택역 맵 OSM에는 `highway=traffic_signals` 노드가 0개라 실제 위치를 쓸 수 없음):
    /// 열린 진입로가 3개 이상이고, 두 축 모두에 **tertiary 이상** 도로 진입로가 있는 교차로(간선끼리 만나는 곳)만
    /// 신호를 둔다. 이면도로가 간선에 붙는 T자 교차로나 이면도로끼리의 교차로는 신호 없음.
    /// 주도로 축 = 차선이 가장 많은 진입로(동률이면 arc 번호가 작은 것)와 45° 안쪽인 진입로, 나머지는 교차 축.
    /// 녹색 시간은 두 축의 차선 수 비율로 나눈다.
    fn rebuild_signals(&mut self) {
        let mut incoming: Vec<Vec<u32>> = vec![Vec::new(); self.nodes.len()];
        for (i, a) in self.arcs.iter().enumerate() {
            if let Some(a) = a {
                if !a.removed {
                    incoming[a.head as usize].push(i as u32);
                }
            }
        }
        const COS45: f64 = std::f64::consts::FRAC_1_SQRT_2;
        let usable = SIGNAL_CYCLE_MS - 2 * LOST_TIME_PER_PHASE_MS;
        for (n, arcs) in incoming.into_iter().enumerate() {
            let mut signal = None;
            if arcs.len() >= 3 {
                let lanes = |a: u32| self.arc(a).lanes.len() as u64;
                let major = *arcs
                    .iter()
                    .max_by_key(|&&a| (lanes(a), Reverse(a)))
                    .expect("진입로 있음");
                let m = self.arc(major).dir_in;
                let mut phase_of = Vec::new();
                let mut lane_sum = [0u64; 2];
                let mut has_major = [false; 2];
                for &a in &arcs {
                    let d = self.arc(a).dir_in;
                    let p = if (d.0 * m.0 + d.1 * m.1).abs() >= COS45 {
                        0
                    } else {
                        1
                    };
                    lane_sum[p as usize] += lanes(a);
                    has_major[p as usize] |= self.arc(a).major;
                    phase_of.push((a, p));
                }
                if lane_sum[1] > 0 && has_major[0] && has_major[1] {
                    let g0 = (usable * lane_sum[0] / (lane_sum[0] + lane_sum[1]))
                        .clamp(MIN_GREEN_MS, usable - MIN_GREEN_MS);
                    signal = Some(Signal {
                        phase_of,
                        green_ms: [g0, usable - g0],
                        // 교차로마다 신호가 한꺼번에 바뀌지 않게 어긋나게 둔다 (가정)
                        offset_ms: (n as u64 * 7_919) % SIGNAL_CYCLE_MS,
                    });
                }
            }
            self.nodes[n].signal = signal;
        }
    }

    /// 노드의 신호 (없으면 None).
    pub fn signal(&self, node: u32) -> Option<&Signal> {
        self.nodes
            .get(node as usize)
            .and_then(|n| n.signal.as_ref())
    }

    /// 신호가 있는 교차로 수.
    pub fn signalized_nodes(&self) -> usize {
        self.nodes.iter().filter(|n| n.signal.is_some()).count()
    }

    /// 엣지의 두 방향을 없앤 것으로 표시한다. 그 위의 차량은 끝까지 빠져나가고, 새로 들어가지 못한다.
    pub fn remove_edge(&mut self, edge: u32) {
        for arc in [edge * 2, edge * 2 + 1] {
            if let Some(a) = self.arcs.get_mut(arc as usize).and_then(Option::as_mut) {
                a.removed = true;
            }
        }
        self.rebuild_signals();
    }

    /// 아직 도착하지 않은 차량의 남은 경로를 다음 결정 지점(지금 차선의 끝 노드, 출발 전이면
    /// 출발 노드)부터 다시 찾는다. `only_if_uses`를 주면 남은 경로에 그 arc가 있는 차량만.
    /// 경로가 바뀐 차량 수를 돌려준다. 새 경로가 없으면 원래 경로를 두고, 교차로에서 다시 찾는다.
    pub fn reroute(&mut self, only_if_uses: Option<&[u32]>) -> u32 {
        let mut changed = 0;
        for v in 0..self.vehicles.len() {
            let veh = &self.vehicles[v];
            if veh.state == VehicleState::Arrived {
                continue;
            }
            let keep = veh.step.map_or(0, |s| s + 1);
            let remaining = &veh.route[keep..];
            if remaining.is_empty() {
                continue;
            }
            if let Some(arcs) = only_if_uses {
                if !remaining.iter().any(|a| arcs.contains(a)) {
                    continue;
                }
            }
            let (node, came) = match veh.step {
                Some(s) => (self.arc(veh.route[s]).head, Some(veh.route[s])),
                None => (veh.origin, None),
            };
            let Some(new) = self.route_now(node, veh.dest, came) else {
                continue;
            };
            if new != remaining {
                let veh = &mut self.vehicles[v];
                veh.route.truncate(keep);
                veh.route.extend(new);
                changed += 1;
            }
        }
        // 기다리던 차량이 새 경로로 갈 수 있게 교차로를 다시 본다
        let waiting: VecDeque<u32> = (0..self.nodes.len() as u32)
            .filter(|&n| !self.nodes[n as usize].waiting.is_empty())
            .collect();
        self.process(waiting);
        changed
    }

    pub fn now_ms(&self) -> u64 {
        self.now_ms
    }

    fn arc(&self, a: u32) -> &ArcInfo {
        self.arcs[a as usize].as_ref().expect("존재하는 arc")
    }

    fn arc_open(&self, a: u32) -> bool {
        self.arcs[a as usize]
            .as_ref()
            .is_some_and(|x| !x.removed && x.lanes.iter().any(|&l| !self.lanes[l as usize].blocked))
    }

    /// 차선을 막거나 연다. 이미 들어간 차량은 그대로 빠져나가고, 새 차량은 들어가지 못한다.
    pub fn set_lane_blocked(&mut self, lane: u32, blocked: bool) {
        self.lanes[lane as usize].blocked = blocked;
        if !blocked {
            let tail = self.arc(self.lanes[lane as usize].arc).tail;
            self.process(VecDeque::from([tail]));
        }
    }

    /// 차선에 있는 차량 수 (교차로를 지나 들어오는 중인 차량 포함).
    pub fn lane_occupancy(&self, lane: u32) -> usize {
        self.lanes[lane as usize].queue.len()
    }

    pub fn lane_capacity(&self, lane: u32) -> u32 {
        self.lanes[lane as usize].capacity
    }

    /// 자유 흐름 속도 기준 최단 시간 경로. `came_from`이 있으면 그 arc의 역방향(유턴)은
    /// 다른 길이 없을 때만 쓴다. 동률이면 arc 번호가 작은 쪽.
    pub fn route(&self, from: u32, dest: u32, came_from: Option<u32>) -> Option<Vec<u32>> {
        self.route_with(from, dest, came_from, false, false)
    }

    /// 지금 혼잡을 반영한 최단 시간 경로 (출발할 때와 막혀서 다시 찾을 때 쓴다).
    /// arc 비용 = 자유 흐름 + 끝 교차로 신호의 평균 대기 + 지금 차선 대기열의 지연 (`arc_cost`).
    pub fn route_now(&self, from: u32, dest: u32, came_from: Option<u32>) -> Option<Vec<u32>> {
        self.route_with(from, dest, came_from, true, false)
    }

    /// 일시적으로 막힌 차선(침수 등)을 열린 것으로 보고, 없앤 도로만 빼고 길이 있는지.
    fn reachable_ignoring_blocks(&self, from: u32, dest: u32, came_from: Option<u32>) -> bool {
        self.route_with(from, dest, came_from, false, true)
            .is_some()
    }

    /// arc 하나를 지나는 예상 시간 (ms, 정수 연산).
    /// - 자유 흐름: 교차로 통과 + 주행 시간
    /// - 혼잡 반영(`congestion`): 끝 교차로가 신호면 평균 신호 대기 r²/(2C)
    ///   (r = 그 진입로의 적색 시간, C = 주기, 주기 동안 고르게 도착한다고 볼 때 적색에 도착할 확률 r/C ×
    ///   평균 대기 r/2), 그리고 지금 차선 대기열 지연 = 대기 차량 ÷ 차선 수 × 포화 차두시간 ÷ 녹색 비율
    fn arc_cost(&self, b: u32, congestion: bool) -> u64 {
        let a = self.arc(b);
        let mut c = INTERSECTION_MS + a.travel_ms;
        if !congestion {
            return c;
        }
        let (green, cycle) = match self.nodes[a.head as usize]
            .signal
            .as_ref()
            .and_then(|s| s.phase(b).map(|p| s.green_ms[p as usize]))
        {
            Some(g) => (g, SIGNAL_CYCLE_MS),
            None => (1, 1),
        };
        let red = cycle - green;
        c += red * red / (2 * cycle);
        let lanes = a.lanes.len().max(1) as u64;
        let queued: u64 = a
            .lanes
            .iter()
            .map(|&l| self.lanes[l as usize].queue.len() as u64)
            .sum();
        c += queued * SATURATION_HEADWAY_MS * cycle / (lanes * green);
        c
    }

    fn route_with(
        &self,
        from: u32,
        dest: u32,
        came_from: Option<u32>,
        congestion: bool,
        ignore_blocks: bool,
    ) -> Option<Vec<u32>> {
        if from == dest {
            return None;
        }
        let n = self.arcs.len();
        let mut dist = vec![u64::MAX; n];
        let mut prev = vec![u32::MAX; n];
        let mut heap = BinaryHeap::new();
        for b in self.allowed_next(from, came_from, ignore_blocks) {
            let c = self.arc_cost(b, congestion);
            if c < dist[b as usize] {
                dist[b as usize] = c;
                heap.push(Reverse((c, b)));
            }
        }
        while let Some(Reverse((c, a))) = heap.pop() {
            if c > dist[a as usize] {
                continue;
            }
            let head = self.arc(a).head;
            if head == dest {
                let mut path = vec![a];
                let mut cur = a;
                while prev[cur as usize] != u32::MAX {
                    cur = prev[cur as usize];
                    path.push(cur);
                }
                path.reverse();
                return Some(path);
            }
            for b in self.allowed_next(head, Some(a), ignore_blocks) {
                let nc = c + self.arc_cost(b, congestion);
                if nc < dist[b as usize] {
                    dist[b as usize] = nc;
                    prev[b as usize] = a;
                    heap.push(Reverse((nc, b)));
                }
            }
        }
        None
    }

    /// 노드에서 나갈 수 있는 열린 arc. 들어온 arc의 유턴은 막다른 곳에서만 허용.
    fn allowed_next(&self, node: u32, came_from: Option<u32>, ignore_blocks: bool) -> Vec<u32> {
        let open: Vec<u32> = self.node_out[node as usize]
            .iter()
            .copied()
            .filter(|&b| {
                if ignore_blocks {
                    self.arcs[b as usize].as_ref().is_some_and(|a| !a.removed)
                } else {
                    self.arc_open(b)
                }
            })
            .collect();
        match came_from {
            Some(a) if open.iter().any(|&b| b != reverse(a)) => {
                open.into_iter().filter(|&b| b != reverse(a)).collect()
            }
            _ => open,
        }
    }

    /// 차량 한 대를 예약한다. 경로가 없으면 None.
    pub fn add_trip(&mut self, origin: u32, dest: u32, depart_ms: u64) -> Option<u32> {
        let route = self.route(origin, dest, None)?;
        let id = self.vehicles.len() as u32;
        self.vehicles.push(Vehicle {
            id,
            origin,
            dest,
            depart_ms,
            arrive_ms: None,
            state: VehicleState::Scheduled,
            route,
            step: None,
            lane: None,
            trace: Vec::new(),
            lane_start_ms: 0,
            lane_travel_ms: 0,
            wait_since_ms: 0,
            reroute_at_ms: 0,
            at_end: false,
        });
        self.schedule(depart_ms.max(self.now_ms), Event::Depart(id));
        Some(id)
    }

    fn schedule(&mut self, t: u64, e: Event) {
        self.seq += 1;
        self.events.push(Reverse((t, self.seq, e)));
    }

    /// `t_ms`까지(포함) 이벤트를 처리한다.
    pub fn run_until(&mut self, t_ms: u64) {
        while let Some(&Reverse((t, _, e))) = self.events.peek() {
            if t > t_ms {
                break;
            }
            self.events.pop();
            self.now_ms = t;
            self.handle(e);
        }
        self.now_ms = self.now_ms.max(t_ms);
    }

    pub fn pending_events(&self) -> usize {
        self.events.len()
    }

    fn handle(&mut self, e: Event) {
        let mut work = VecDeque::new();
        match e {
            Event::Depart(v) => {
                let (origin, dest) = {
                    let veh = &self.vehicles[v as usize];
                    (veh.origin, veh.dest)
                };
                if let Some(r) = self.route_now(origin, dest, None) {
                    self.vehicles[v as usize].route = r;
                }
                self.vehicles[v as usize].state = VehicleState::WaitingToEnter;
                self.vehicles[v as usize].wait_since_ms = self.now_ms;
                self.vehicles[v as usize].reroute_at_ms = self.now_ms + REROUTE_WAIT_MS;
                self.vehicles[v as usize].reroute_at_ms = self.now_ms + REROUTE_WAIT_MS;
                self.nodes[origin as usize].waiting.push(v);
                work.push_back(origin);
            }
            Event::ReachEnd(v) => {
                self.vehicles[v as usize].at_end = true;
                let lane = self.vehicles[v as usize].lane.expect("차선 위 차량");
                if self.lanes[lane as usize].queue.front() == Some(&v) {
                    self.on_front(v, &mut work);
                } else {
                    self.vehicles[v as usize].state = VehicleState::Queued;
                }
            }
            Event::Wake(n) => {
                let node = &mut self.nodes[n as usize];
                if node.wake_at == Some(self.now_ms) {
                    node.wake_at = None;
                }
                work.push_back(n);
            }
        }
        self.process(work);
    }

    fn process(&mut self, mut work: VecDeque<u32>) {
        while let Some(n) = work.pop_front() {
            self.try_advance(n, &mut work);
        }
    }

    /// 차선 맨 앞에 와서 끝에 닿은 차량: 목적지면 도착, 아니면 교차로 통과 요청.
    fn on_front(&mut self, v: u32, work: &mut VecDeque<u32>) {
        let veh = &self.vehicles[v as usize];
        let step = veh.step.expect("주행 중");
        let arc = veh.route[step];
        let head = self.arc(arc).head;
        if step + 1 == veh.route.len() {
            let lane = veh.lane.expect("차선 위 차량");
            let veh = &mut self.vehicles[v as usize];
            veh.state = VehicleState::Arrived;
            veh.arrive_ms = Some(self.now_ms);
            veh.lane = None;
            self.leave_lane(lane, v, work);
        } else {
            self.vehicles[v as usize].state = VehicleState::WaitingToAdvance;
            self.vehicles[v as usize].wait_since_ms = self.now_ms;
            self.nodes[head as usize].waiting.push(v);
            work.push_back(head);
        }
    }

    fn leave_lane(&mut self, lane: u32, v: u32, work: &mut VecDeque<u32>) {
        let q = &mut self.lanes[lane as usize].queue;
        debug_assert_eq!(q.front(), Some(&v));
        q.pop_front();
        // 자리가 났으니 이 차선으로 들어오려는 교차로를 다시 본다
        work.push_back(self.arc(self.lanes[lane as usize].arc).tail);
        if let Some(&f) = self.lanes[lane as usize].queue.front() {
            if self.vehicles[f as usize].at_end {
                self.on_front(f, work);
            }
        }
    }

    /// 갈 길이 없어진 차량을 도로에서 뺀다.
    fn cancel(&mut self, n: u32, v: u32, work: &mut VecDeque<u32>) {
        if let Some(pos) = self.nodes[n as usize].waiting.iter().position(|&x| x == v) {
            self.nodes[n as usize].waiting.remove(pos);
        }
        if let Some(l) = self.vehicles[v as usize].lane {
            self.leave_lane(l, v, work);
        }
        let veh = &mut self.vehicles[v as usize];
        veh.lane = None;
        veh.state = VehicleState::Cancelled;
        self.cancelled += 1;
    }

    /// 교착 해소용: 용량과 무관하게 막히지 않은 차선 중 차량이 가장 적은 것.
    fn pick_lane_over_capacity(&self, arc: u32) -> Option<u32> {
        if self.arc(arc).removed {
            return None;
        }
        self.arc(arc)
            .lanes
            .iter()
            .copied()
            .filter(|&l| !self.lanes[l as usize].blocked)
            .min_by_key(|&l| (self.lanes[l as usize].queue.len(), l))
    }

    /// 들어갈 차선: 막히지 않고 자리가 있는 차선 중 차량이 가장 적은 것, 동률이면 번호가 작은 것.
    fn pick_lane(&self, arc: u32) -> Option<u32> {
        self.arc(arc)
            .lanes
            .iter()
            .copied()
            .filter(|&l| {
                let s = &self.lanes[l as usize];
                !s.blocked && (s.queue.len() as u32) < s.capacity
            })
            .filter(|_| !self.arc(arc).removed)
            .min_by_key(|&l| (self.lanes[l as usize].queue.len(), l))
    }

    fn next_arc(&self, v: u32) -> u32 {
        let veh = &self.vehicles[v as usize];
        veh.route[veh.step.map_or(0, |s| s + 1)]
    }

    /// 기다리는 차량을 먼저 온 순서로 보며, 갈 수 있는 차량은 모두 보낸다. 조건:
    /// ① 들어온 차선(출발이면 출발 노드)의 차두시간이 지남 ② 신호가 있으면 그 진입로가 녹색
    /// ③ 다음 arc에 자리가 있음. ①·②로 못 가면 그 시각에 다시 보도록 Wake를 예약한다.
    fn try_advance(&mut self, n: u32, work: &mut VecDeque<u32>) {
        let now = self.now_ms;
        let mut wake: Option<u64> = None;
        let waiting = self.nodes[n as usize].waiting.clone();
        for v in waiting {
            let mut next = self.next_arc(v);
            if !self.arc_open(next) {
                // 다음 arc가 모두 막힘 → 이 노드에서 경로를 다시 찾는다
                let veh = &self.vehicles[v as usize];
                let came = veh.step.map(|s| veh.route[s]);
                let Some(new) = self.route_now(n, veh.dest, came) else {
                    // 지금 갈 길이 없음. 막힌 차선(일시적)이 다시 열리면 갈 수 있으면 기다리고,
                    // 없앤 도로 때문에 영영 갈 수 없으면 통행을 포기하고 도로에서 빠진다.
                    let dest = veh.dest;
                    if !self.reachable_ignoring_blocks(n, dest, came) {
                        self.cancel(n, v, work);
                    }
                    continue;
                };
                let veh = &mut self.vehicles[v as usize];
                let keep = veh.step.map_or(0, |s| s + 1);
                veh.route.truncate(keep);
                veh.route.extend(new);
                next = self.next_arc(v);
            }
            let old_lane = self.vehicles[v as usize].lane;
            let free_at = match old_lane {
                Some(l) => self.lanes[l as usize].free_at,
                None => self.nodes[n as usize].spawn_free_at,
            };
            if free_at > now {
                wake = Some(wake.map_or(free_at, |w| w.min(free_at)));
                continue;
            }
            if let (Some(l), Some(sig)) = (old_lane, &self.nodes[n as usize].signal) {
                let in_arc = self.lanes[l as usize].arc;
                if !sig.is_green(in_arc, now) {
                    let g = sig.next_green(in_arc, now);
                    wake = Some(wake.map_or(g, |w| w.min(g)));
                    continue;
                }
            }
            let lane = match self.pick_lane(next) {
                Some(l) => Some(l),
                None => {
                    // 다음 차선이 가득 참
                    let (since, reroute_at) = {
                        let veh = &self.vehicles[v as usize];
                        (veh.wait_since_ms, veh.reroute_at_ms)
                    };
                    if now >= reroute_at {
                        // ① 오래 기다렸으면 지금 혼잡을 반영해 다른 길을 찾는다
                        let veh = &self.vehicles[v as usize];
                        let came = veh.step.map(|s| veh.route[s]);
                        if let Some(new) = self.route_now(n, veh.dest, came) {
                            if new[0] != next {
                                let veh = &mut self.vehicles[v as usize];
                                let keep = veh.step.map_or(0, |s| s + 1);
                                veh.route.truncate(keep);
                                veh.route.extend(new);
                                next = self.next_arc(v);
                            }
                        }
                        self.vehicles[v as usize].reroute_at_ms = now + REROUTE_WAIT_MS;
                    }
                    let mut lane = self.pick_lane(next);
                    if lane.is_none() && now >= since + JAM_RELIEF_WAIT_MS {
                        // ② 그래도 막혀 있으면 용량을 넘겨서라도 들어간다 (교착 해소)
                        lane = self.pick_lane_over_capacity(next);
                        if lane.is_some() {
                            self.jam_relief += 1;
                        }
                    }
                    if lane.is_none() {
                        let r = self.vehicles[v as usize].reroute_at_ms;
                        let t = r.min(since + JAM_RELIEF_WAIT_MS).max(now + 1);
                        wake = Some(wake.map_or(t, |w| w.min(t)));
                    }
                    lane
                }
            };
            let Some(lane) = lane else {
                continue;
            };

            let pos = self.nodes[n as usize]
                .waiting
                .iter()
                .position(|&x| x == v)
                .expect("대기 중");
            self.nodes[n as usize].waiting.remove(pos);
            match old_lane {
                Some(l) => self.lanes[l as usize].free_at = now + SATURATION_HEADWAY_MS,
                None => self.nodes[n as usize].spawn_free_at = now + SATURATION_HEADWAY_MS,
            }
            if let Some(old) = old_lane {
                self.leave_lane(old, v, work);
            }
            let t = now + INTERSECTION_MS;
            self.lanes[lane as usize].queue.push_back(v);
            let travel = self.arc(next).travel_ms;
            let veh = &mut self.vehicles[v as usize];
            veh.step = Some(veh.step.map_or(0, |s| s + 1));
            veh.lane = Some(lane);
            veh.trace.push(lane);
            veh.at_end = false;
            veh.state = VehicleState::Crossing;
            veh.lane_start_ms = t;
            veh.lane_travel_ms = travel;
            self.schedule(t + travel, Event::ReachEnd(v));
        }
        if let Some(t) = wake {
            let node = &mut self.nodes[n as usize];
            if node.wake_at.is_none_or(|w| t < w) {
                node.wake_at = Some(t);
                self.schedule(t, Event::Wake(n));
            }
        }
    }

    /// 차선 위 차량의 위치: (차량, 차선, 차선 시작점에서의 거리 m).
    /// 주행 중이면 들어온 뒤 지난 시간에 비례해 나아가고, 앞차가 있으면 그 뒤
    /// (대기열 순번 × 차량 간격)에 선다. 교차로를 지나는 2초 동안은 차선 시작점에 있다.
    /// 화면 표시용이며 시뮬레이션 결과에는 영향이 없다.
    pub fn lane_positions(&self, g: &RoadGraph) -> Vec<(u32, u32, f64)> {
        let mut out = Vec::new();
        for (l, s) in self.lanes.iter().enumerate() {
            let len = g.lanes[l].length_m;
            for (idx, &v) in s.queue.iter().enumerate() {
                let veh = &self.vehicles[v as usize];
                let elapsed = self.now_ms.saturating_sub(veh.lane_start_ms) as f64;
                let progress = if veh.at_end || veh.lane_travel_ms == 0 {
                    1.0
                } else {
                    (elapsed / veh.lane_travel_ms as f64).clamp(0.0, 1.0)
                };
                let behind = len - idx as f64 * dongne_world::road_graph::VEHICLE_SPACING_M;
                out.push((v, l as u32, (progress * len).min(behind).max(0.0)));
            }
        }
        out.sort_unstable_by_key(|&(v, _, _)| v);
        out
    }

    pub fn stats(&self) -> TrafficStats {
        let times: Vec<f64> = self
            .vehicles
            .iter()
            .filter_map(|v| v.arrive_ms.map(|a| (a - v.depart_ms) as f64 / 1000.0))
            .collect();
        TrafficStats {
            vehicles: self.vehicles.len(),
            arrived: times.len(),
            mean_travel_s: if times.is_empty() {
                0.0
            } else {
                times.iter().sum::<f64>() / times.len() as f64
            },
            max_travel_s: times.iter().copied().fold(0.0, f64::max),
            now_s: self.now_ms as f64 / 1000.0,
            jam_relief: self.jam_relief,
            cancelled: self.cancelled,
        }
    }
}
