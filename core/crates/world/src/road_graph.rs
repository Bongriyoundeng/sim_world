//! `roads.geojson`에서 차량 도로 그래프(노드·엣지·차선)를 만든다.
//!
//! ADR 0004의 차선 대기열 모델 입력이다. 엣지마다 방향별 차선 수를 두고,
//! 차선마다 길이와 대기열 용량(대수)을 계산한다.
//!
//! 결정론: 같은 `roads.geojson`이면 같은 노드·엣지·차선 ID가 나온다.
//! 노드 ID는 스냅 좌표 (x, y) 순, 엣지 ID는 (osm_id, 피처의 첫 좌표, 선을 따라간 순서) 순이다.

use std::collections::{BTreeMap, BTreeSet};

use serde_json::Value;

/// 정점을 같은 노드로 보는 허용 오차 (m). 좌표를 이 간격의 격자로 반올림해 비교한다.
pub const SNAP_M: f64 = 0.5;
/// 구역 경계에 붙은 끝점으로 보는 거리 (m).
pub const BOUNDARY_EPS_M: f64 = 0.05;
/// 대기열 용량 계산에 쓰는 차량 한 대가 차지하는 길이 (m, 가정: 승용차 길이 + 정지 간격).
pub const VEHICLE_SPACING_M: f64 = 7.5;

/// 그래프에 넣는 차량 통행 도로 유형.
pub fn is_vehicle_road(highway: &str) -> bool {
    matches!(
        highway,
        "motorway"
            | "trunk"
            | "primary"
            | "secondary"
            | "tertiary"
            | "unclassified"
            | "residential"
            | "living_street"
            | "service"
            | "motorway_link"
            | "trunk_link"
            | "primary_link"
            | "secondary_link"
            | "tertiary_link"
    )
}

/// `lanes` 태그가 없을 때의 양방향 합계 차선 수 (가정, docs/design/research-log.md#road-lanes).
/// gis/landcover.py의 `DEFAULT_LANES`와 같은 값이다.
pub fn default_lanes(highway: &str) -> u32 {
    match highway {
        "motorway" | "trunk" | "primary" => 4,
        "secondary" | "tertiary" | "unclassified" | "residential" => 2,
        _ => 1, // *_link, living_street, service
    }
}

/// 자유 흐름 속도 (km/h, 가정). OSM `maxspeed`는 아직 맵 패키지에 없다.
pub fn free_speed_kmh(highway: &str) -> f64 {
    match highway.trim_end_matches("_link") {
        "motorway" => 100.0,
        "trunk" => 80.0,
        "primary" => 60.0,
        "secondary" => 50.0,
        "tertiary" => 50.0,
        "unclassified" | "residential" => 30.0,
        "living_street" => 20.0,
        _ => 20.0, // service
    }
}

/// 방향별 차선 수 (정방향 = 좌표 순서, 역방향).
/// 일방통행이면 모든 차선이 정방향. 양방향이면 반씩 나누고(홀수면 정방향에 하나 더), 각 방향 최소 1.
pub fn lanes_by_direction(highway: &str, lanes: Option<u32>, oneway: bool) -> (u32, u32) {
    let total = lanes
        .filter(|&n| n > 0)
        .unwrap_or_else(|| default_lanes(highway));
    if oneway {
        (total, 0)
    } else {
        (total.div_ceil(2).max(1), (total / 2).max(1))
    }
}

#[derive(Debug, Clone, PartialEq)]
pub struct Node {
    pub id: u32,
    pub x: f64,
    pub y: f64,
    /// 구역 경계에서 잘린 도로 끝점
    pub boundary: bool,
}

#[derive(Debug, Clone, PartialEq)]
pub struct Edge {
    pub id: u32,
    pub from: u32,
    pub to: u32,
    pub osm_id: i64,
    pub highway: String,
    pub name: Option<String>,
    pub length_m: f64,
    /// from → to 방향 차선 수
    pub lanes_forward: u32,
    /// to → from 방향 차선 수 (일방통행이면 0)
    pub lanes_backward: u32,
    pub free_speed_kmh: f64,
    /// from에서 to까지의 선 (로컬 미터). 클라이언트 보간용.
    pub points: Vec<(f64, f64)>,
    /// 명령으로 없앤 엣지. ID를 유지하려고 목록에서 지우지 않는다.
    pub removed: bool,
}

/// 차선 대기열 모델의 차선 하나.
#[derive(Debug, Clone, PartialEq)]
pub struct Lane {
    pub id: u32,
    pub edge: u32,
    /// true = 엣지 from → to
    pub forward: bool,
    /// 같은 방향 안에서 0 = 가장 안쪽(중앙선 쪽)
    pub index: u32,
    pub length_m: f64,
    /// 대기열에 들어갈 수 있는 차량 수 (최소 1)
    pub capacity: u32,
}

#[derive(Debug, Clone, Default, PartialEq)]
pub struct RoadGraph {
    pub nodes: Vec<Node>,
    pub edges: Vec<Edge>,
    pub lanes: Vec<Lane>,
    /// 그래프에서 뺀 도로 피처 수 (highway 유형별)
    pub excluded: BTreeMap<String, u32>,
}

#[derive(Debug, Clone, PartialEq)]
pub struct GraphStats {
    pub nodes: usize,
    pub edges: usize,
    pub lanes: usize,
    pub boundary_nodes: usize,
    pub lane_km: f64,
    /// 가장 큰 강연결 요소에 속한 노드 비율 (방향 그래프)
    pub largest_scc_ratio: f64,
    pub excluded: BTreeMap<String, u32>,
}

#[derive(Debug)]
pub enum GraphError {
    Parse(serde_json::Error),
    Format(String),
}

impl std::fmt::Display for GraphError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            GraphError::Parse(e) => write!(f, "roads.geojson 형식 오류: {e}"),
            GraphError::Format(m) => write!(f, "roads.geojson 형식 오류: {m}"),
        }
    }
}

impl std::error::Error for GraphError {}

struct Road {
    osm_id: i64,
    highway: String,
    name: Option<String>,
    lanes: Option<u32>,
    oneway: bool,
    points: Vec<(f64, f64)>,
}

type Key = (i64, i64);

fn key(p: (f64, f64)) -> Key {
    ((p.0 / SNAP_M).round() as i64, (p.1 / SNAP_M).round() as i64)
}

fn dist(a: (f64, f64), b: (f64, f64)) -> f64 {
    (a.0 - b.0).hypot(a.1 - b.1)
}

fn parse_roads(json: &str, excluded: &mut BTreeMap<String, u32>) -> Result<Vec<Road>, GraphError> {
    let v: Value = serde_json::from_str(json).map_err(GraphError::Parse)?;
    let features = v["features"]
        .as_array()
        .ok_or_else(|| GraphError::Format("features 배열이 없음".into()))?;
    let mut roads = Vec::new();
    for f in features {
        let p = &f["properties"];
        let highway = p["highway"].as_str().unwrap_or("").to_string();
        if !is_vehicle_road(&highway) {
            *excluded.entry(highway).or_insert(0) += 1;
            continue;
        }
        if f["geometry"]["type"].as_str() != Some("LineString") {
            return Err(GraphError::Format("도로는 LineString이어야 함".into()));
        }
        let mut points: Vec<(f64, f64)> = Vec::new();
        for c in f["geometry"]["coordinates"]
            .as_array()
            .into_iter()
            .flatten()
        {
            let (Some(x), Some(y)) = (c[0].as_f64(), c[1].as_f64()) else {
                return Err(GraphError::Format("좌표가 숫자가 아님".into()));
            };
            if points.last().map(|&q| key(q)) != Some(key((x, y))) {
                points.push((x, y));
            }
        }
        if points.len() < 2 {
            continue;
        }
        roads.push(Road {
            osm_id: p["osm_id"].as_i64().unwrap_or(0),
            highway,
            name: p["name"].as_str().map(str::to_string),
            lanes: p["lanes"].as_u64().map(|n| n as u32),
            oneway: p["oneway"].as_bool().unwrap_or(false),
            points,
        });
    }
    // 결정론: 입력 피처 순서와 무관하게 osm_id, 첫 좌표 순으로 처리
    roads.sort_by(|a, b| {
        (
            a.osm_id,
            key(a.points[0]),
            key(a.points[a.points.len() - 1]),
        )
            .cmp(&(
                b.osm_id,
                key(b.points[0]),
                key(b.points[b.points.len() - 1]),
            ))
    });
    Ok(roads)
}

impl RoadGraph {
    /// `roads.geojson` 문자열과 구역 크기(m)로 그래프를 만든다.
    pub fn from_geojson(json: &str, width_m: f64, height_m: f64) -> Result<Self, GraphError> {
        let mut excluded = BTreeMap::new();
        let roads = parse_roads(json, &mut excluded)?;

        // 1. 노드가 될 정점: 끝점, 두 번 이상 쓰인 정점
        let mut uses: BTreeMap<Key, u32> = BTreeMap::new();
        let mut ends: BTreeSet<Key> = BTreeSet::new();
        let mut pos: BTreeMap<Key, (f64, f64)> = BTreeMap::new();
        for r in &roads {
            for &p in &r.points {
                *uses.entry(key(p)).or_insert(0) += 1;
                pos.entry(key(p)).or_insert(p);
            }
            ends.insert(key(r.points[0]));
            ends.insert(key(r.points[r.points.len() - 1]));
        }
        let node_keys: BTreeSet<Key> = uses
            .iter()
            .filter(|(k, &n)| n >= 2 || ends.contains(k))
            .map(|(&k, _)| k)
            .collect();

        let (hw, hh) = (width_m / 2.0, height_m / 2.0);
        let mut node_id: BTreeMap<Key, u32> = BTreeMap::new();
        let mut nodes = Vec::with_capacity(node_keys.len());
        for (i, k) in node_keys.iter().enumerate() {
            let (x, y) = pos[k];
            let boundary = ends.contains(k)
                && (x.abs() >= hw - BOUNDARY_EPS_M || y.abs() >= hh - BOUNDARY_EPS_M);
            node_id.insert(*k, i as u32);
            nodes.push(Node {
                id: i as u32,
                x,
                y,
                boundary,
            });
        }

        // 2. 엣지: 노드 사이 구간
        let mut edges = Vec::new();
        for r in &roads {
            let (fwd, bwd) = lanes_by_direction(&r.highway, r.lanes, r.oneway);
            let mut seg = vec![r.points[0]];
            for &p in &r.points[1..] {
                seg.push(p);
                if let Some(&to) = node_id.get(&key(p)) {
                    let from = node_id[&key(seg[0])];
                    let length_m = seg.windows(2).map(|w| dist(w[0], w[1])).sum();
                    edges.push(Edge {
                        id: edges.len() as u32,
                        from,
                        to,
                        osm_id: r.osm_id,
                        highway: r.highway.clone(),
                        name: r.name.clone(),
                        length_m,
                        lanes_forward: fwd,
                        lanes_backward: bwd,
                        free_speed_kmh: free_speed_kmh(&r.highway),
                        points: std::mem::take(&mut seg),
                        removed: false,
                    });
                    seg.push(p);
                }
            }
        }

        // 3. 차선
        let mut lanes = Vec::new();
        for e in &edges {
            let capacity = ((e.length_m / VEHICLE_SPACING_M).floor() as u32).max(1);
            for (forward, n) in [(true, e.lanes_forward), (false, e.lanes_backward)] {
                for index in 0..n {
                    lanes.push(Lane {
                        id: lanes.len() as u32,
                        edge: e.id,
                        forward,
                        index,
                        length_m: e.length_m,
                        capacity,
                    });
                }
            }
        }

        Ok(RoadGraph {
            nodes,
            edges,
            lanes,
            excluded,
        })
    }

    /// 엣지 위 한 점: 진행 방향 시작점에서 `dist_m`만큼 간 곳을, 진행 방향 오른쪽으로
    /// 차선 `lane_index`(0 = 중앙선 쪽) 중심만큼 옮긴 좌표와 진행 방향(라디안, x축 기준).
    /// `lane_width_m`은 표시용 차로 폭. 우측 통행 기준.
    pub fn lane_point(
        &self,
        edge: u32,
        forward: bool,
        lane_index: u32,
        dist_m: f64,
        lane_width_m: f64,
    ) -> Option<(f64, f64, f64)> {
        let e = self.edges.get(edge as usize)?;
        let pts: Vec<(f64, f64)> = if forward {
            e.points.clone()
        } else {
            e.points.iter().rev().copied().collect()
        };
        let mut left = dist_m.max(0.0);
        let mut seg = (pts[0], pts[1]);
        for w in pts.windows(2) {
            seg = (w[0], w[1]);
            let l = dist(w[0], w[1]);
            if left <= l {
                break;
            }
            left -= l;
        }
        let ((x0, y0), (x1, y1)) = seg;
        let l = dist(seg.0, seg.1).max(1e-9);
        let t = (left / l).min(1.0);
        let (dx, dy) = ((x1 - x0) / l, (y1 - y0) / l);
        // 양방향 도로는 중앙선(엣지 선)에서 오른쪽으로, 일방통행은 도로 가운데를 기준으로 놓는다
        let offset = if e.lanes_forward > 0 && e.lanes_backward > 0 {
            (lane_index as f64 + 0.5) * lane_width_m
        } else {
            let n = (e.lanes_forward + e.lanes_backward) as f64;
            (lane_index as f64 + 0.5 - n / 2.0) * lane_width_m
        };
        Some((
            x0 + dx * l * t + dy * offset,
            y0 + dy * l * t - dx * offset,
            dy.atan2(dx),
        ))
    }

    /// 좌표에서 `max_m` 안의 가장 가까운 (없애지 않은) 엣지.
    pub fn nearest_edge(&self, x: f64, y: f64, max_m: f64) -> Option<u32> {
        let seg_dist = |a: (f64, f64), b: (f64, f64)| {
            let (vx, vy) = (b.0 - a.0, b.1 - a.1);
            let len2 = vx * vx + vy * vy;
            let t = if len2 > 0.0 {
                (((x - a.0) * vx + (y - a.1) * vy) / len2).clamp(0.0, 1.0)
            } else {
                0.0
            };
            (a.0 + vx * t - x).hypot(a.1 + vy * t - y)
        };
        self.edges
            .iter()
            .filter(|e| !e.removed)
            .map(|e| {
                let d = e
                    .points
                    .windows(2)
                    .map(|w| seg_dist(w[0], w[1]))
                    .fold(f64::INFINITY, f64::min);
                (d, e.id)
            })
            .filter(|&(d, _)| d <= max_m)
            .min_by(|a, b| a.0.total_cmp(&b.0).then(a.1.cmp(&b.1)))
            .map(|(_, id)| id)
    }

    /// 좌표에서 `radius_m` 안의 가장 가까운 노드 (동률이면 번호가 작은 노드).
    pub fn nearest_node(&self, x: f64, y: f64, radius_m: f64) -> Option<u32> {
        self.nodes
            .iter()
            .map(|n| ((n.x - x).hypot(n.y - y), n.id))
            .filter(|&(d, _)| d <= radius_m)
            .min_by(|a, b| a.0.total_cmp(&b.0).then(a.1.cmp(&b.1)))
            .map(|(_, id)| id)
    }

    /// 새 노드를 추가하고 ID를 돌려준다.
    pub fn add_node(&mut self, x: f64, y: f64) -> u32 {
        let id = self.nodes.len() as u32;
        self.nodes.push(Node {
            id,
            x,
            y,
            boundary: false,
        });
        id
    }

    /// 두 노드를 잇는 직선 도로 엣지와 그 차선을 추가하고 엣지 ID를 돌려준다.
    /// 검증(유형, 차선 수, 길이)은 호출하는 쪽이 한다.
    pub fn add_edge(
        &mut self,
        from: u32,
        to: u32,
        highway: &str,
        lanes: Option<u32>,
        oneway: bool,
    ) -> u32 {
        let a = &self.nodes[from as usize];
        let b = &self.nodes[to as usize];
        let points = vec![(a.x, a.y), (b.x, b.y)];
        let length_m = dist(points[0], points[1]);
        let (fwd, bwd) = lanes_by_direction(highway, lanes, oneway);
        let id = self.edges.len() as u32;
        self.edges.push(Edge {
            id,
            from,
            to,
            osm_id: 0,
            highway: highway.to_string(),
            name: None,
            length_m,
            lanes_forward: fwd,
            lanes_backward: bwd,
            free_speed_kmh: free_speed_kmh(highway),
            points,
            removed: false,
        });
        let capacity = ((length_m / VEHICLE_SPACING_M).floor() as u32).max(1);
        for (forward, n) in [(true, fwd), (false, bwd)] {
            for index in 0..n {
                self.lanes.push(Lane {
                    id: self.lanes.len() as u32,
                    edge: id,
                    forward,
                    index,
                    length_m,
                    capacity,
                });
            }
        }
        id
    }

    /// 엣지를 없앤 것으로 표시한다. 이미 없거나 없는 번호면 false.
    pub fn remove_edge(&mut self, id: u32) -> bool {
        match self.edges.get_mut(id as usize) {
            Some(e) if !e.removed => {
                e.removed = true;
                true
            }
            _ => false,
        }
    }

    /// 방향 그래프의 인접 목록 (차선이 있는 방향만).
    pub fn successors(&self) -> Vec<Vec<u32>> {
        let mut adj = vec![Vec::new(); self.nodes.len()];
        for e in self.edges.iter().filter(|e| !e.removed) {
            if e.lanes_forward > 0 {
                adj[e.from as usize].push(e.to);
            }
            if e.lanes_backward > 0 {
                adj[e.to as usize].push(e.from);
            }
        }
        adj
    }

    /// 강연결 요소별 노드 목록 (Tarjan, 재귀 없이).
    pub fn strongly_connected_components(&self) -> Vec<Vec<u32>> {
        let adj = self.successors();
        let n = adj.len();
        let mut index = vec![u32::MAX; n];
        let mut low = vec![0u32; n];
        let mut on_stack = vec![false; n];
        let mut stack = Vec::new();
        let mut comps = Vec::new();
        let mut next = 0u32;
        for start in 0..n {
            if index[start] != u32::MAX {
                continue;
            }
            let mut call: Vec<(usize, usize)> = vec![(start, 0)];
            index[start] = next;
            low[start] = next;
            next += 1;
            stack.push(start);
            on_stack[start] = true;
            while let Some(&mut (v, ref mut i)) = call.last_mut() {
                if *i < adj[v].len() {
                    let w = adj[v][*i] as usize;
                    *i += 1;
                    if index[w] == u32::MAX {
                        index[w] = next;
                        low[w] = next;
                        next += 1;
                        stack.push(w);
                        on_stack[w] = true;
                        call.push((w, 0));
                    } else if on_stack[w] {
                        low[v] = low[v].min(index[w]);
                    }
                } else {
                    call.pop();
                    if let Some(&(parent, _)) = call.last() {
                        low[parent] = low[parent].min(low[v]);
                    }
                    if low[v] == index[v] {
                        let mut comp = Vec::new();
                        while let Some(w) = stack.pop() {
                            on_stack[w] = false;
                            comp.push(w as u32);
                            if w == v {
                                break;
                            }
                        }
                        comp.sort_unstable();
                        comps.push(comp);
                    }
                }
            }
        }
        comps
    }

    pub fn stats(&self) -> GraphStats {
        let largest = self
            .strongly_connected_components()
            .iter()
            .map(Vec::len)
            .max()
            .unwrap_or(0);
        GraphStats {
            nodes: self.nodes.len(),
            edges: self.edges.iter().filter(|e| !e.removed).count(),
            lanes: self
                .lanes
                .iter()
                .filter(|l| !self.edges[l.edge as usize].removed)
                .count(),
            boundary_nodes: self.nodes.iter().filter(|n| n.boundary).count(),
            lane_km: self
                .lanes
                .iter()
                .filter(|l| !self.edges[l.edge as usize].removed)
                .map(|l| l.length_m)
                .sum::<f64>()
                / 1000.0,
            largest_scc_ratio: if self.nodes.is_empty() {
                0.0
            } else {
                largest as f64 / self.nodes.len() as f64
            },
            excluded: self.excluded.clone(),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn feature(
        osm_id: i64,
        highway: &str,
        lanes: Option<u32>,
        oneway: bool,
        pts: &[(f64, f64)],
    ) -> String {
        let coords: Vec<String> = pts.iter().map(|(x, y)| format!("[{x},{y}]")).collect();
        let lanes = lanes.map_or("null".to_string(), |n| n.to_string());
        format!(
            r#"{{"type":"Feature","properties":{{"osm_id":{osm_id},"highway":"{highway}","name":null,"lanes":{lanes},"oneway":{oneway}}},"geometry":{{"type":"LineString","coordinates":[{}]}}}}"#,
            coords.join(",")
        )
    }

    fn collection(features: &[String]) -> String {
        format!(
            r#"{{"type":"FeatureCollection","features":[{}]}}"#,
            features.join(",")
        )
    }

    /// 십자 교차로: 가로 도로(양방향) + 세로 도로(일방통행, 북→남), 가운데에서 만남.
    /// 가로 도로는 서쪽 구역 경계(x = -100)에서 잘려 있다.
    fn crossing() -> String {
        collection(&[
            feature(
                10,
                "secondary",
                Some(4),
                false,
                &[(-100.0, 0.0), (0.0, 0.0), (50.0, 0.0)],
            ),
            feature(
                20,
                "residential",
                None,
                true,
                &[(0.0, 50.0), (0.2, 0.1), (0.0, -50.0)],
            ),
            feature(30, "footway", None, false, &[(-10.0, -10.0), (10.0, -10.0)]),
        ])
    }

    #[test]
    fn crossing_nodes_and_edges() {
        let g = RoadGraph::from_geojson(&crossing(), 200.0, 200.0).unwrap();
        // 노드: 가로 양끝 2 + 세로 양끝 2 + 교차점 1 (0.2,0.1은 0.5m 스냅으로 (0,0)에 붙음)
        assert_eq!(g.nodes.len(), 5);
        assert_eq!(g.edges.len(), 4);
        assert_eq!(g.excluded.get("footway"), Some(&1));

        let west = g.nodes.iter().find(|n| n.x == -100.0).unwrap();
        assert!(west.boundary);
        assert_eq!(g.nodes.iter().filter(|n| n.boundary).count(), 1);

        let center = g
            .nodes
            .iter()
            .find(|n| n.x == 0.0 && n.y == 0.0)
            .unwrap()
            .id;
        let degree = g
            .edges
            .iter()
            .filter(|e| e.from == center || e.to == center)
            .count();
        assert_eq!(degree, 4);

        let h: Vec<&Edge> = g.edges.iter().filter(|e| e.osm_id == 10).collect();
        assert_eq!(h.len(), 2);
        assert!((h[0].length_m - 100.0).abs() < 1e-9 && (h[1].length_m - 50.0).abs() < 1e-9);
        assert!(h
            .iter()
            .all(|e| e.lanes_forward == 2 && e.lanes_backward == 2));

        let v: Vec<&Edge> = g.edges.iter().filter(|e| e.osm_id == 20).collect();
        assert!(v
            .iter()
            .all(|e| e.lanes_forward == 2 && e.lanes_backward == 0)); // 일방통행 기본 2차로
        assert!(v[0].points.first().unwrap().1 > v[0].points.last().unwrap().1);
        // 북 → 남
    }

    #[test]
    fn lanes_and_capacity() {
        let g = RoadGraph::from_geojson(&crossing(), 200.0, 200.0).unwrap();
        // 가로 2구간 × 4차선 + 세로 2구간 × 2차선
        assert_eq!(g.lanes.len(), 2 * 4 + 2 * 2);
        let l = g
            .lanes
            .iter()
            .find(|l| g.edges[l.edge as usize].length_m == 100.0)
            .unwrap();
        assert_eq!(l.capacity, (100.0 / VEHICLE_SPACING_M) as u32);
        let s = g.stats();
        let expected = (100.0 * 4.0
            + 50.0 * 4.0
            + g.edges
                .iter()
                .filter(|e| e.osm_id == 20)
                .map(|e| e.length_m * 2.0)
                .sum::<f64>())
            / 1000.0;
        assert!((s.lane_km - expected).abs() < 1e-9);
    }

    #[test]
    fn oneway_breaks_strong_connectivity() {
        let g = RoadGraph::from_geojson(&crossing(), 200.0, 200.0).unwrap();
        let s = g.stats();
        // 가로 도로 3노드는 서로 오갈 수 있다. 일방통행 세로 도로의 북쪽 끝은 나가기만, 남쪽 끝은 들어오기만 해서 따로 떨어진다
        assert!((s.largest_scc_ratio - 3.0 / 5.0).abs() < 1e-9);
    }

    #[test]
    fn deterministic_regardless_of_feature_order() {
        let a = RoadGraph::from_geojson(&crossing(), 200.0, 200.0).unwrap();
        let mut fs = vec![
            feature(30, "footway", None, false, &[(-10.0, -10.0), (10.0, -10.0)]),
            feature(
                20,
                "residential",
                None,
                true,
                &[(0.0, 50.0), (0.2, 0.1), (0.0, -50.0)],
            ),
            feature(
                10,
                "secondary",
                Some(4),
                false,
                &[(-100.0, 0.0), (0.0, 0.0), (50.0, 0.0)],
            ),
        ];
        let b = RoadGraph::from_geojson(&collection(&fs), 200.0, 200.0).unwrap();
        assert_eq!(a, b);
        fs.reverse();
        assert_eq!(
            a,
            RoadGraph::from_geojson(&collection(&fs), 200.0, 200.0).unwrap()
        );
    }

    #[test]
    fn lane_split_rules() {
        assert_eq!(lanes_by_direction("residential", None, false), (1, 1));
        assert_eq!(lanes_by_direction("primary", None, false), (2, 2));
        assert_eq!(lanes_by_direction("secondary", Some(3), false), (2, 1));
        assert_eq!(lanes_by_direction("service", None, false), (1, 1));
        assert_eq!(lanes_by_direction("tertiary", Some(3), true), (3, 0));
        assert_eq!(lanes_by_direction("tertiary", Some(0), true), (2, 0));
    }

    #[test]
    fn lane_point_and_nearest_edge() {
        let g = RoadGraph::from_geojson(&crossing(), 200.0, 200.0).unwrap();
        // 가로 도로 서쪽 구간 (-100,0)→(0,0), 양방향 2+2차선
        let w = g
            .edges
            .iter()
            .find(|e| e.osm_id == 10 && e.length_m == 100.0)
            .unwrap();
        let (x, y, h) = g.lane_point(w.id, true, 0, 25.0, 3.0).unwrap();
        assert!((x - -75.0).abs() < 1e-9 && (y - -1.5).abs() < 1e-9 && h.abs() < 1e-9);
        // 역방향 1번 차선: 동→서로 25 m, 진행 방향 오른쪽(북쪽)으로 4.5 m
        let (x, y, _) = g.lane_point(w.id, false, 1, 25.0, 3.0).unwrap();
        assert!((x - -25.0).abs() < 1e-9 && (y - 4.5).abs() < 1e-9);
        // 길이를 넘으면 끝점
        let (x, _, _) = g.lane_point(w.id, true, 0, 500.0, 3.0).unwrap();
        assert!((x - 0.0).abs() < 1e-9);

        assert_eq!(g.nearest_edge(-50.0, 3.0, 5.0), Some(w.id));
        assert_eq!(g.nearest_edge(-50.0, 30.0, 5.0), None);
    }

    #[test]
    fn loop_road_keeps_single_node_edge() {
        let g = RoadGraph::from_geojson(
            &collection(&[feature(
                1,
                "service",
                None,
                true,
                &[(0.0, 0.0), (10.0, 0.0), (10.0, 10.0), (0.0, 0.0)],
            )]),
            100.0,
            100.0,
        )
        .unwrap();
        assert_eq!(g.nodes.len(), 1);
        assert_eq!(g.edges.len(), 1);
        assert_eq!(g.edges[0].from, g.edges[0].to);
    }
}
