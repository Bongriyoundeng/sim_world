//! 작은 합성 도로망으로 차선 대기열·교차로 순서·막힌 차선을 검사한다.

use dongne_sim_core::traffic::{Traffic, VehicleState, INTERSECTION_MS};
use dongne_world::RoadGraph;

fn feature(osm_id: i64, lanes: Option<u32>, oneway: bool, pts: &[(f64, f64)]) -> String {
    let coords: Vec<String> = pts.iter().map(|(x, y)| format!("[{x},{y}]")).collect();
    let lanes = lanes.map_or("null".to_string(), |n| n.to_string());
    format!(
        r#"{{"type":"Feature","properties":{{"osm_id":{osm_id},"highway":"residential","name":null,"lanes":{lanes},"oneway":{oneway}}},"geometry":{{"type":"LineString","coordinates":[{}]}}}}"#,
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
fn intersection_serves_first_come_one_at_a_time() {
    // 서쪽·북쪽에서 X로 들어와 동쪽으로 나가는 일방통행, 두 진입로 길이 같음
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
    let v1 = t.add_trip(w, e, 0).unwrap() as usize;
    let v2 = t.add_trip(n, e, 500).unwrap() as usize; // 0.5초 늦게 출발 → X에 늦게 닿음
    t.run_until(120_000);
    let (a1, a2) = (
        t.vehicles[v1].arrive_ms.unwrap(),
        t.vehicles[v2].arrive_ms.unwrap(),
    );
    assert!(a1 < a2);
    // 출구 차선이 2개여도 교차로는 한 번에 한 대: v2는 v1이 다 지나간 뒤(2초 뒤) 들어간다
    assert_eq!(a2 - a1, INTERSECTION_MS);
    // 출구에서는 비어 있는 다른 차선을 고른다
    assert_ne!(t.vehicles[v1].trace.last(), t.vehicles[v2].trace.last());
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
