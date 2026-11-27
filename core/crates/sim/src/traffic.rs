//! 이벤트 기반 차선 대기열 교통 모델 (ADR 0004).
//!
//! - 시간은 정수 밀리초. 이벤트는 (시각, 순번) 순서로 처리해 결정론을 지킨다.
//! - 차량은 차선 주행(Crossing) → 차선 끝 대기열(Queued) → 맨 앞에서 교차로 통과 대기
//!   (WaitingToAdvance) → 다음 차선 순서로 움직인다. 가감속과 추월은 없다.
//! - 차선마다 대기열 용량(도로 그래프의 길이 ÷ 차량 간격)이 있고, 가득 차면 들어가지 못한다.
//! - 교차로는 신호 없이 선착순, 한 번에 한 대가 `INTERSECTION_MS` 동안 지나간다 (가정).
//!
//! 가정과 파라미터: docs/design/research-log.md#traffic-queue

use std::cmp::Reverse;
use std::collections::{BinaryHeap, VecDeque};

use dongne_world::RoadGraph;

/// 교차로 한 대 통과 시간 (ms, 가정).
pub const INTERSECTION_MS: u64 = 2_000;

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
    /// 교차로를 지나던 차량이 다 지나감
    NodeFree(u32),
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
}

#[derive(Debug, Clone)]
struct ArcInfo {
    tail: u32,
    head: u32,
    travel_ms: u64,
    lanes: Vec<u32>,
    /// 명령으로 없앤 도로
    removed: bool,
}

#[derive(Debug, Clone)]
struct LaneState {
    arc: u32,
    capacity: u32,
    blocked: bool,
    queue: VecDeque<u32>,
}

#[derive(Debug, Clone, Default)]
struct NodeState {
    busy: bool,
    /// 통과를 기다리는 차량 (요청 순서)
    waiting: Vec<u32>,
}

#[derive(Debug, Clone, PartialEq)]
pub struct TrafficStats {
    pub vehicles: usize,
    pub arrived: usize,
    pub mean_travel_s: f64,
    pub max_travel_s: f64,
    pub now_s: f64,
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
                self.arcs[arc as usize] = Some(ArcInfo {
                    tail,
                    head,
                    travel_ms,
                    lanes: Vec::new(),
                    removed: e.removed,
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
            });
        }
    }

    /// 엣지의 두 방향을 없앤 것으로 표시한다. 그 위의 차량은 끝까지 빠져나가고, 새로 들어가지 못한다.
    pub fn remove_edge(&mut self, edge: u32) {
        for arc in [edge * 2, edge * 2 + 1] {
            if let Some(a) = self.arcs.get_mut(arc as usize).and_then(Option::as_mut) {
                a.removed = true;
            }
        }
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
            let Some(new) = self.route(node, veh.dest, came) else {
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
        if from == dest {
            return None;
        }
        let n = self.arcs.len();
        let mut dist = vec![u64::MAX; n];
        let mut prev = vec![u32::MAX; n];
        let mut heap = BinaryHeap::new();
        for b in self.allowed_next(from, came_from) {
            let c = INTERSECTION_MS + self.arc(b).travel_ms;
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
            for b in self.allowed_next(head, Some(a)) {
                let nc = c + INTERSECTION_MS + self.arc(b).travel_ms;
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
    fn allowed_next(&self, node: u32, came_from: Option<u32>) -> Vec<u32> {
        let open: Vec<u32> = self.node_out[node as usize]
            .iter()
            .copied()
            .filter(|&b| self.arc_open(b))
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
                let origin = self.vehicles[v as usize].origin;
                self.vehicles[v as usize].state = VehicleState::WaitingToEnter;
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
            Event::NodeFree(n) => {
                self.nodes[n as usize].busy = false;
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

    /// 노드가 비어 있으면, 기다리는 차량 중 먼저 온 순서로 갈 수 있는 첫 차량을 통과시킨다.
    fn try_advance(&mut self, n: u32, work: &mut VecDeque<u32>) {
        if self.nodes[n as usize].busy {
            return;
        }
        let waiting = self.nodes[n as usize].waiting.clone();
        for (i, &v) in waiting.iter().enumerate() {
            let mut next = self.next_arc(v);
            if !self.arc_open(next) {
                // 다음 arc가 모두 막힘 → 이 노드에서 경로를 다시 찾는다
                let veh = &self.vehicles[v as usize];
                let came = veh.step.map(|s| veh.route[s]);
                let Some(new) = self.route(n, veh.dest, came) else {
                    continue;
                };
                let veh = &mut self.vehicles[v as usize];
                let keep = veh.step.map_or(0, |s| s + 1);
                veh.route.truncate(keep);
                veh.route.extend(new);
                next = self.next_arc(v);
            }
            let Some(lane) = self.pick_lane(next) else {
                continue;
            };
            self.nodes[n as usize].waiting.remove(i);
            self.nodes[n as usize].busy = true;
            let t = self.now_ms + INTERSECTION_MS;
            self.schedule(t, Event::NodeFree(n));

            if let Some(old) = self.vehicles[v as usize].lane {
                self.leave_lane(old, v, work);
            }
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
            return;
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
        }
    }
}
