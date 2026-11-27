//! 시뮬레이션 틱 루프. 고정 틱, 결정론적.

use std::cmp::Reverse;

pub use dongne_api::{Command, CommandOutcome, CommandResult, RoadEnd};
use dongne_world::road_graph::is_vehicle_road;
use dongne_world::World;

pub mod demand;
pub mod snapshot;
pub mod traffic;
pub use demand::{Commuter, DemandParams, DemandSummary};
pub use traffic::{Rng, Traffic, TrafficStats, Vehicle, VehicleState};

/// `add_commute` 결과.
#[derive(Debug, Clone, PartialEq, Default)]
pub struct CommuteReport {
    pub demand: DemandSummary,
    /// 예약한 차량 통행 수 (출근 + 퇴근)
    pub trips: u32,
    /// 집과 일터가 같은 노드라 차량 통행이 없는 통근자 수
    pub same_node: u32,
    /// 경로를 못 찾은 통행 수
    pub unroutable: u32,
}

/// 틱 하나의 시뮬레이션 시간 (ms, 가정). 교통 이벤트는 틱 경계까지 처리된다.
pub const TICK_MS: u64 = 100;
/// 도로 추가 명령의 좌표 끝을 기존 노드에 붙이는 반경 (m, 가정).
pub const ROAD_SNAP_RADIUS_M: f64 = 5.0;
/// 새 도로의 최소 길이 (m, 가정).
pub const MIN_ROAD_LENGTH_M: f64 = 1.0;

/// 시뮬레이션 상태.
#[derive(Debug)]
pub struct Simulation {
    pub world: World,
    pub traffic: Traffic,
    tick: u64,
    pending: Vec<(u64, Command)>,
    next_command: u64,
    /// 처리한 명령의 결과 (처리 순서)
    pub outcomes: Vec<CommandOutcome>,
}

impl Simulation {
    pub fn new(world: World) -> Self {
        let traffic = Traffic::new(&world.road_graph);
        Self {
            world,
            traffic,
            tick: 0,
            pending: Vec::new(),
            next_command: 0,
            outcomes: Vec::new(),
        }
    }

    /// 지금까지 진행한 틱 수.
    pub fn tick(&self) -> u64 {
        self.tick
    }

    /// 명령을 넣는다. 다음 `step`이 시작될 때(지금 틱 경계에서) 넣은 순서대로 적용되고,
    /// 결과는 `outcomes`에 남는다. 돌려주는 번호는 결과의 `id`와 같다.
    pub fn submit(&mut self, command: Command) -> u64 {
        let id = self.next_command;
        self.next_command += 1;
        self.pending.push((id, command));
        id
    }

    /// n틱 진행한다. 먼저 쌓인 명령을 적용하고, 그 시각까지의 교통 이벤트를 처리한다.
    pub fn step(&mut self, n_ticks: u64) {
        for (id, command) in std::mem::take(&mut self.pending) {
            let result = self.apply(&command);
            self.outcomes.push(CommandOutcome {
                id,
                tick: self.tick,
                result,
            });
        }
        self.tick += n_ticks;
        self.traffic.run_until(self.tick * TICK_MS);
    }

    fn apply(&mut self, command: &Command) -> CommandResult {
        match self.try_apply(command) {
            Ok(r) => r,
            Err(reason) => CommandResult::Rejected { reason },
        }
    }

    fn try_apply(&mut self, command: &Command) -> Result<CommandResult, String> {
        match command {
            Command::AddRoad {
                from,
                to,
                highway,
                lanes,
                oneway,
            } => {
                if !is_vehicle_road(highway) {
                    return Err(format!("차량 도로 유형이 아님: {highway}"));
                }
                if *lanes == Some(0) {
                    return Err("차선 수는 1 이상이어야 함".into());
                }
                let a = self.resolve_end(from)?;
                let b = self.resolve_end(to)?;
                let length = (a.1 .0 - b.1 .0).hypot(a.1 .1 - b.1 .1);
                if a.0.is_some() && a.0 == b.0 || length < MIN_ROAD_LENGTH_M {
                    return Err(format!(
                        "두 끝이 너무 가까움 ({length:.2} m < {MIN_ROAD_LENGTH_M} m)"
                    ));
                }
                let g = &mut self.world.road_graph;
                let mut new_nodes = Vec::new();
                let mut node = |end: (Option<u32>, (f64, f64)), g: &mut dongne_world::RoadGraph| {
                    end.0.unwrap_or_else(|| {
                        let id = g.add_node(end.1 .0, end.1 .1);
                        new_nodes.push(id);
                        id
                    })
                };
                let na = node(a, g);
                let nb = node(b, g);
                let edge = g.add_edge(na, nb, highway, *lanes, *oneway);
                self.traffic.sync(&self.world.road_graph);
                let rerouted = self.traffic.reroute(None);
                Ok(CommandResult::Applied {
                    edge: Some(edge),
                    new_nodes,
                    cells: 0,
                    rerouted,
                })
            }
            Command::RemoveRoad { edge } => {
                if !self.world.road_graph.remove_edge(*edge) {
                    return Err(format!("없거나 이미 없앤 엣지: {edge}"));
                }
                self.traffic.remove_edge(*edge);
                let rerouted = self.traffic.reroute(Some(&[edge * 2, edge * 2 + 1]));
                Ok(CommandResult::Applied {
                    edge: Some(*edge),
                    new_nodes: Vec::new(),
                    cells: 0,
                    rerouted,
                })
            }
            Command::SetZone { area, zone } => {
                let cells = self.world.zones.resolve(area)?;
                self.world.zones.set(&cells, *zone);
                Ok(CommandResult::Applied {
                    edge: None,
                    new_nodes: Vec::new(),
                    cells: cells.len() as u32,
                    rerouted: 0,
                })
            }
        }
    }

    /// 도로 끝을 (기존 노드, 좌표)로 바꾼다. 좌표가 기존 노드 가까이면 그 노드를 쓴다.
    fn resolve_end(&self, end: &RoadEnd) -> Result<(Option<u32>, (f64, f64)), String> {
        let g = &self.world.road_graph;
        match *end {
            RoadEnd::Node(id) => {
                let n = g
                    .nodes
                    .get(id as usize)
                    .ok_or_else(|| format!("없는 노드: {id}"))?;
                Ok((Some(id), (n.x, n.y)))
            }
            RoadEnd::Point(p) => {
                let e = &self.world.manifest.extent_m;
                if !p.x.is_finite()
                    || !p.y.is_finite()
                    || p.x.abs() > e.width / 2.0
                    || p.y.abs() > e.height / 2.0
                {
                    return Err(format!("좌표 ({}, {})가 맵 범위 밖", p.x, p.y));
                }
                match g.nearest_node(p.x, p.y, ROAD_SNAP_RADIUS_M) {
                    Some(id) => {
                        let n = &g.nodes[id as usize];
                        Ok((Some(id), (n.x, n.y)))
                    }
                    None => Ok((None, (p.x, p.y))),
                }
            }
        }
    }

    /// 지금 구역 맵에서 통근자를 만들고 출근(집 → 일터)·퇴근(일터 → 집) 차량 통행을 예약한다.
    pub fn add_commute(&mut self, params: &DemandParams, seed: u64) -> CommuteReport {
        let (commuters, demand) =
            demand::generate_commuters(&self.world.zones, &self.world.road_graph, params, seed);
        let mut report = CommuteReport {
            demand,
            ..Default::default()
        };
        for c in &commuters {
            if c.home_node == c.work_node {
                report.same_node += 1;
                continue;
            }
            for (o, d, t) in [
                (c.home_node, c.work_node, c.leave_home_ms),
                (c.work_node, c.home_node, c.leave_work_ms),
            ] {
                if self.traffic.add_trip(o, d, t).is_some() {
                    report.trips += 1;
                } else {
                    report.unroutable += 1;
                }
            }
        }
        report
    }

    /// 가장 큰 강연결 요소 안에서 무작위 출발·도착 노드로 차량 `n`대를 예약한다.
    /// 출발 시각은 `[0, depart_window_ms)`에서 고르게 뽑는다. 같은 시드면 같은 차량이 나온다.
    /// 예약한 차량 수를 돌려준다 (경로를 못 찾는 쌍은 다시 뽑고, 시도 횟수를 넘으면 그만둔다).
    /// 엣지별로 실제로 지나간 차량 수 (같은 차량은 한 번만). 많은 순, 동률이면 엣지 번호 순.
    pub fn edge_usage(&self) -> Vec<(u32, u32)> {
        let lanes = &self.world.road_graph.lanes;
        let mut count = std::collections::BTreeMap::<u32, u32>::new();
        for v in &self.traffic.vehicles {
            let mut edges: Vec<u32> = v.trace.iter().map(|&l| lanes[l as usize].edge).collect();
            edges.sort_unstable();
            edges.dedup();
            for e in edges {
                *count.entry(e).or_insert(0) += 1;
            }
        }
        let mut out: Vec<(u32, u32)> = count.into_iter().collect();
        out.sort_by_key(|&(e, n)| (Reverse(n), e));
        out
    }

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
