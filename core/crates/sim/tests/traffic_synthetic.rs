//! 작은 합성 도로망으로 차선 대기열·교차로 순서·막힌 차선을 검사한다.

use dongne_sim_core::traffic::{
    Traffic, VehicleState, INTERSECTION_MS, LOST_TIME_PER_PHASE_MS, SATURATION_HEADWAY_MS,
    SIGNAL_CYCLE_MS,
};
use dongne_world::RoadGraph;

fn feature(osm_id: i64, lanes: Option<u32>, oneway: bool, pts: &[(f64, f64)]) -> String {
    let coords: Vec<String> = pts.iter().map(|(x, y)| format!("[{x},{y}]")).collect();
    let lanes = lanes.map_or("null".to_string(), |n| n.to_string());
    format!(
        r#"{{"type":"Feature","properties":{{"osm_id":{osm_id},"highway":"residential","name":null,"lanes":{lanes},"oneway":{oneway}}},"geometry":{{"type":"LineString","coordinates":[{}]}}}}"#,
        coords.join(",")
    )
}

fn feature_hw(
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

fn graph(features: &[String]) -> RoadGraph {
    let json = format!(
        r#"{{"type":"FeatureCollection","features":[{}]}}"#,
        features.join(",")
    );
    RoadGraph::from_geojson(&json, 2000.0, 2000.0).unwrap()
}

fn node(g: &RoadGraph, x: f64, y: f64) -> u32 {
    g.nodes.iter().find(|n| n.x == x && n.y == y).unwrap().id
}

fn lanes_of(g: &RoadGraph, osm_id: i64) -> Vec<u32> {
    let edges: Vec<u32> = g
        .edges
        .iter()
        .filter(|e| e.osm_id == osm_id)
        .map(|e| e.id)
        .collect();
    g.lanes
        .iter()
        .filter(|l| edges.contains(&l.edge))
        .map(|l| l.id)
        .collect()
}

#[test]
fn lane_capacity_is_never_exceeded() {
    // A →(15 m, 1차로, 용량 2대)→ B →(500 m)→ C
    let g = graph(&[
        feature(1, Some(1), true, &[(0.0, 0.0), (15.0, 0.0)]),
        feature(2, Some(1), true, &[(15.0, 0.0), (515.0, 0.0)]),
    ]);
    let short = lanes_of(&g, 1)[0];
    let (a, c) = (node(&g, 0.0, 0.0), node(&g, 515.0, 0.0));
    let mut t = Traffic::new(&g);
    assert_eq!(t.lane_capacity(short), 2);
    for _ in 0..6 {
        t.add_trip(a, c, 0).unwrap();
    }
    let mut max_occ = 0;
    for step in 1..=20_000u64 {
        t.run_until(step * 100);
        max_occ = max_occ.max(t.lane_occupancy(short));
        if t.stats().arrived == 6 {
            break;
        }
    }
    assert_eq!(t.stats().arrived, 6);
    assert!(max_occ <= 2 && max_occ > 0);
    // 같은 출발점에서 차례로 들어갔으니 도착도 차례대로, 교차로 통과 간격 이상 벌어진다
    let arrive: Vec<u64> = t.vehicles.iter().map(|v| v.arrive_ms.unwrap()).collect();
    assert!(arrive.windows(2).all(|w| w[1] >= w[0] + INTERSECTION_MS));
}

#[test]
fn unsignalized_merge_lets_each_approach_go_independently() {
    // 서쪽·북쪽에서 X로 들어와 동쪽(2차로)으로 나가는 일방통행. 진입로 2개 → 신호 없음
    let g = graph(&[
        feature(1, Some(1), true, &[(-100.0, 0.0), (0.0, 0.0)]),
        feature(2, Some(1), true, &[(0.0, 100.0), (0.0, 0.0)]),
        feature(3, Some(2), true, &[(0.0, 0.0), (100.0, 0.0)]),
    ]);
    let (w, n, e) = (
        node(&g, -100.0, 0.0),
        node(&g, 0.0, 100.0),
        node(&g, 100.0, 0.0),
    );
    let mut t = Traffic::new(&g);
    assert_eq!(t.signalized_nodes(), 0);
    let v1 = t.add_trip(w, e, 0).unwrap() as usize;
    let v2 = t.add_trip(n, e, 500).unwrap() as usize; // 0.5초 늦게 출발
    t.run_until(120_000);
    let (a1, a2) = (
        t.vehicles[v1].arrive_ms.unwrap(),
        t.vehicles[v2].arrive_ms.unwrap(),
    );
    // 진입로마다 따로 떠나므로 출발 간격(0.5초) 그대로 도착하고, 출구에서는 다른 차선을 고른다
    assert_eq!(a2 - a1, 500);
    assert_ne!(t.vehicles[v1].trace.last(), t.vehicles[v2].trace.last());
}

#[test]
fn same_lane_departs_at_saturation_headway() {
    // A → X(진입 1차로) → B, 두 대가 X 앞에 같이 줄을 섬: 두 번째는 포화 차두시간(2초) 뒤에 떠난다
    let g = graph(&[
        feature(1, Some(1), true, &[(0.0, 0.0), (100.0, 0.0)]),
        feature(2, Some(2), true, &[(100.0, 0.0), (300.0, 0.0)]),
    ]);
    let (a, b) = (node(&g, 0.0, 0.0), node(&g, 300.0, 0.0));
    let mut t = Traffic::new(&g);
    let v1 = t.add_trip(a, b, 0).unwrap() as usize;
    let v2 = t.add_trip(a, b, 0).unwrap() as usize;
    t.run_until(300_000);
    let (a1, a2) = (
        t.vehicles[v1].arrive_ms.unwrap(),
        t.vehicles[v2].arrive_ms.unwrap(),
    );
    assert_eq!(a2 - a1, SATURATION_HEADWAY_MS);
}

/// 4갈래 교차로 X(0,0): 동서 2+2차로, 남북 1+1차로, 모두 tertiary(간선끼리 만나 신호 있음)
fn crossroads() -> RoadGraph {
    let two_way =
        |id, lanes, pts: &[(f64, f64)]| feature_hw(id, "tertiary", Some(lanes), false, pts);
    graph(&[
        two_way(1, 4, &[(-200.0, 0.0), (0.0, 0.0)]),
        two_way(2, 4, &[(0.0, 0.0), (200.0, 0.0)]),
        two_way(3, 2, &[(0.0, 200.0), (0.0, 0.0)]),
        two_way(4, 2, &[(0.0, 0.0), (0.0, -200.0)]),
    ])
}

#[test]
fn four_way_junction_gets_two_phase_signal() {
    let g = crossroads();
    let x = node(&g, 0.0, 0.0);
    let t = Traffic::new(&g);
    assert_eq!(t.signalized_nodes(), 1, "끝점(진입로 1개)은 신호 없음");
    let sig = t.signal(x).unwrap().clone();
    assert_eq!(sig.phase_of.len(), 4);
    // 주도로 축(동서, 진입 차선 2+2)이 현시 0, 남북(1+1)이 현시 1, 녹색은 차선 비율 2:1
    let usable = SIGNAL_CYCLE_MS - 2 * LOST_TIME_PER_PHASE_MS;
    assert_eq!(sig.green_ms, [usable * 4 / 6, usable - usable * 4 / 6]);
    let east_in = g
        .edges
        .iter()
        .find(|e| e.osm_id == 1)
        .map(|e| e.id * 2)
        .unwrap();
    let south_in = g
        .edges
        .iter()
        .find(|e| e.osm_id == 3)
        .map(|e| e.id * 2)
        .unwrap();
    assert_eq!(sig.phase_of.iter().find(|p| p.0 == east_in).unwrap().1, 0);
    assert_eq!(sig.phase_of.iter().find(|p| p.0 == south_in).unwrap().1, 1);
    // 한 주기 안에서 두 현시는 겹치지 않고, 녹색 시간 합은 주기 − 손실 시간
    let (mut g0, mut g1) = (0, 0);
    for ms in (0..SIGNAL_CYCLE_MS).step_by(100) {
        let (a, b) = (sig.is_green(east_in, ms), sig.is_green(south_in, ms));
        assert!(!(a && b));
        g0 += a as u64;
        g1 += b as u64;
    }
    assert_eq!((g0 + g1) * 100, usable);
    // 빨간불이면 다음 녹색 시작까지 기다린다
    let red = (0..SIGNAL_CYCLE_MS)
        .find(|&ms| !sig.is_green(south_in, ms))
        .unwrap();
    let next = sig.next_green(south_in, red);
    assert!(next > red && sig.is_green(south_in, next) && !sig.is_green(south_in, next - 1));
}

#[test]
fn vehicle_on_red_waits_for_its_green() {
    let g = crossroads();
    let (n, s, x) = (
        node(&g, 0.0, 200.0),
        node(&g, 0.0, -200.0),
        node(&g, 0.0, 0.0),
    );
    let mut probe = Traffic::new(&g);
    let south_in = probe.route(n, x, None).unwrap()[0];
    let sig = probe.signal(x).unwrap().clone();
    // 북→남 차량이 X에 닿는 시각 = 출발 + 출발 교차로 2초 + 200 m 주행 (tertiary 50 km/h)
    probe.add_trip(n, s, 0).unwrap();
    probe.run_until(600_000);
    let free_flow_arrival = probe.vehicles[0].arrive_ms.unwrap();
    let reach_x = INTERSECTION_MS + (200.0 / (50.0 / 3.6) * 1000.0f64).ceil() as u64;
    let wait = if sig.is_green(south_in, reach_x) {
        0
    } else {
        sig.next_green(south_in, reach_x) - reach_x
    };
    let travel_after = free_flow_arrival - reach_x - wait;
    // 출발 시각을 바꿔 X에 빨간불일 때 닿게 하면, 도착은 다음 녹색까지 기다린 만큼 늦다
    let red_at = (0..SIGNAL_CYCLE_MS)
        .map(|ms| ms + SIGNAL_CYCLE_MS)
        .find(|&ms| !sig.is_green(south_in, ms) && sig.is_green(south_in, ms + 5_000))
        .unwrap();
    let mut t = Traffic::new(&g);
    t.add_trip(n, s, red_at - reach_x).unwrap();
    t.run_until(900_000);
    let arrive = t.vehicles[0].arrive_ms.unwrap();
    assert_eq!(
        arrive,
        sig.next_green(south_in, red_at) + travel_after,
        "빨간불에서 다음 녹색까지 정확히 기다림"
    );
}

#[test]
fn blocked_lane_is_avoided_and_blocked_road_is_detoured() {
    // A → B 직선(2차로) + 우회로 A → D → B
    let g = graph(&[
        feature(1, Some(2), true, &[(0.0, 0.0), (100.0, 0.0)]),
        feature(
            2,
            Some(1),
            true,
            &[(0.0, 0.0), (0.0, 100.0), (100.0, 100.0)],
        ),
        feature(3, Some(1), true, &[(100.0, 100.0), (100.0, 0.0)]),
    ]);
    let (a, b) = (node(&g, 0.0, 0.0), node(&g, 100.0, 0.0));
    let direct = lanes_of(&g, 1);
    assert_eq!(direct.len(), 2);

    // 한 차선만 막으면 남은 차선으로만 간다
    let mut t = Traffic::new(&g);
    t.set_lane_blocked(direct[0], true);
    for i in 0..4 {
        t.add_trip(a, b, i * 100).unwrap();
    }
    t.run_until(300_000);
    assert!(t.vehicles.iter().all(|v| v.state == VehicleState::Arrived));
    assert!(t.vehicles.iter().all(|v| v.trace == vec![direct[1]]));
    let fast = t.vehicles[0].arrive_ms.unwrap();

    // 두 차선을 다 막으면 우회로로 간다
    let mut t = Traffic::new(&g);
    for &l in &direct {
        t.set_lane_blocked(l, true);
    }
    t.add_trip(a, b, 0).unwrap();
    t.run_until(300_000);
    let v = &t.vehicles[0];
    assert_eq!(v.state, VehicleState::Arrived);
    assert!(v.trace.iter().all(|l| !direct.contains(l)));
    assert!(v.arrive_ms.unwrap() > fast);

    // 경로를 정한 뒤 출발 전에 막혀도 교차로에서 경로를 다시 찾는다
    let mut t = Traffic::new(&g);
    t.add_trip(a, b, 1_000).unwrap();
    for &l in &direct {
        t.set_lane_blocked(l, true);
    }
    t.run_until(300_000);
    assert_eq!(t.vehicles[0].state, VehicleState::Arrived);
    assert!(t.vehicles[0].trace.iter().all(|l| !direct.contains(l)));
}

#[test]
fn unreachable_trip_is_rejected() {
    let g = graph(&[feature(1, Some(1), true, &[(0.0, 0.0), (100.0, 0.0)])]);
    let mut t = Traffic::new(&g);
    // 일방통행 역주행, 같은 노드
    assert!(t
        .add_trip(node(&g, 100.0, 0.0), node(&g, 0.0, 0.0), 0)
        .is_none());
    assert!(t
        .add_trip(node(&g, 0.0, 0.0), node(&g, 0.0, 0.0), 0)
        .is_none());
}

#[test]
fn no_u_turn_unless_dead_end() {
    // 양방향 A – B – C, B에서 남쪽으로 막다른 길 B – D
    let g = graph(&[
        feature(1, Some(2), false, &[(0.0, 0.0), (100.0, 0.0), (200.0, 0.0)]),
        feature(2, Some(2), false, &[(100.0, 0.0), (100.0, -100.0)]),
    ]);
    let (a, b, d) = (
        node(&g, 0.0, 0.0),
        node(&g, 100.0, 0.0),
        node(&g, 100.0, -100.0),
    );
    let t = Traffic::new(&g);
    let r = t.route(a, d, None).unwrap();
    assert_eq!(r.len(), 2); // A→B, B→D

    // A에서 B로 들어온 차량이 A로 돌아가려 할 때, B에서 바로 유턴(B→A)하지 않는다.
    // 다른 길(B→D)이 있으니 그쪽으로 가서 막다른 D에서 돌아온다.
    let a_to_b = r[0];
    let back = t.route(b, a, Some(a_to_b)).unwrap();
    assert_ne!(back[0], a_to_b ^ 1);
    assert_eq!(back.len(), 3); // B→D, D→B(막다른 곳 유턴), B→A

    // 막다른 D에서는 유턴밖에 없으니 허용된다
    let from_d = t.route(d, a, Some(r[1])).unwrap();
    assert_eq!(from_d[0], r[1] ^ 1);
}

#[test]
fn lane_positions_queue_behind_each_other() {
    // A →(100 m)→ B →(100 m)→ C, B→C 차선을 막아 A→B 끝에 줄을 세운다
    let g = graph(&[
        feature(1, Some(1), true, &[(0.0, 0.0), (100.0, 0.0)]),
        feature(2, Some(1), true, &[(100.0, 0.0), (200.0, 0.0)]),
    ]);
    let (a, c) = (node(&g, 0.0, 0.0), node(&g, 200.0, 0.0));
    let mut t = Traffic::new(&g);
    for i in 0..3 {
        t.add_trip(a, c, i * 3_000).unwrap();
    }
    // 5초: 첫 차는 A→B를 달리는 중, 둘째 차는 막 들어와 교차로를 지나는 중(시작점)
    t.run_until(5_000);
    let p = t.lane_positions(&g);
    assert_eq!(p.len(), 2);
    assert!(p[0].2 > 0.0 && p[0].2 < 100.0);
    assert_eq!(p[1].2, 0.0);

    t.set_lane_blocked(lanes_of(&g, 2)[0], true);
    t.run_until(120_000);
    let p = t.lane_positions(&g);
    let d: Vec<f64> = p.iter().map(|x| x.2).collect();
    assert_eq!(p.len(), 3, "세 대 모두 A→B에서 기다림: {p:?}");
    assert_eq!(d, vec![100.0, 92.5, 85.0]);
}

#[test]
fn minor_junctions_have_no_signal() {
    // 이면도로(residential) 4갈래, 그리고 간선(tertiary) 동서축에 이면도로가 붙은 T자 → 둘 다 신호 없음
    let minor = graph(&[
        feature(1, Some(2), false, &[(-200.0, 0.0), (0.0, 0.0)]),
        feature(2, Some(2), false, &[(0.0, 0.0), (200.0, 0.0)]),
        feature(3, Some(2), false, &[(0.0, 200.0), (0.0, 0.0)]),
        feature(4, Some(2), false, &[(0.0, 0.0), (0.0, -200.0)]),
    ]);
    assert_eq!(Traffic::new(&minor).signalized_nodes(), 0);
    let t_junction = graph(&[
        feature_hw(1, "tertiary", Some(2), false, &[(-200.0, 0.0), (0.0, 0.0)]),
        feature_hw(2, "tertiary", Some(2), false, &[(0.0, 0.0), (200.0, 0.0)]),
        feature(3, Some(2), false, &[(0.0, 200.0), (0.0, 0.0)]),
    ]);
    assert_eq!(Traffic::new(&t_junction).signalized_nodes(), 0);
}
