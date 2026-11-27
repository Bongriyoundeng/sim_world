//! 명령(도로 추가·삭제, 구역 지정) 적용과 경로 재계산, 거부, 결정론.

use std::path::PathBuf;

use dongne_api::{Command, CommandResult, Point, RoadEnd, Zone, ZoneArea};
use dongne_sim_core::{Simulation, VehicleState, TICK_MS};
use dongne_world::{load_map, parse_manifest, RoadGraph, World, ZoneMap};

fn feature(osm_id: i64, lanes: u32, pts: &[(f64, f64)]) -> String {
    let coords: Vec<String> = pts.iter().map(|(x, y)| format!("[{x},{y}]")).collect();
    format!(
        r#"{{"type":"Feature","properties":{{"osm_id":{osm_id},"highway":"residential","name":null,"lanes":{lanes},"oneway":true}},"geometry":{{"type":"LineString","coordinates":[{}]}}}}"#,
        coords.join(",")
    )
}

fn world(features: &[String]) -> World {
    let manifest = parse_manifest(
        r#"{"format_version":0,"name":"t","origin":{"lat":0,"lon":0},"crs":"local-meters",
        "extent_m":{"width":2000,"height":2000},
        "terrain":{"file":"terrain.png","resolution_m":30,"min_elevation_m":0,"max_elevation_m":1},
        "attribution":["test"]}"#,
    )
    .unwrap();
    let json = format!(
        r#"{{"type":"FeatureCollection","features":[{}]}}"#,
        features.join(",")
    );
    World {
        road_graph: RoadGraph::from_geojson(&json, 2000.0, 2000.0).unwrap(),
        zones: ZoneMap::new(2000.0, 2000.0, 30.0),
        manifest,
    }
}

/// A(0,0) → B(300,0) 직선 + 우회로 A → D(0,300) → E(300,300) → B
fn detour_world(with_direct: bool) -> World {
    let mut fs = vec![
        feature(2, 1, &[(0.0, 0.0), (0.0, 300.0), (300.0, 300.0)]),
        feature(3, 1, &[(300.0, 300.0), (300.0, 0.0)]),
    ];
    if with_direct {
        fs.push(feature(1, 2, &[(0.0, 0.0), (300.0, 0.0)]));
    }
    world(&fs)
}

fn node(sim: &Simulation, x: f64, y: f64) -> u32 {
    sim.world
        .road_graph
        .nearest_node(x, y, 0.01)
        .expect("노드가 있어야 함")
}

fn run_to_end(sim: &mut Simulation) {
    for _ in 0..2_000 {
        sim.step(100);
        if sim
            .traffic
            .vehicles
            .iter()
            .all(|v| v.state == VehicleState::Arrived)
        {
            return;
        }
    }
    panic!("시간 안에 모두 도착하지 못함");
}

fn edge_lanes(sim: &Simulation, edge: u32) -> Vec<u32> {
    sim.world
        .road_graph
        .lanes
        .iter()
        .filter(|l| l.edge == edge)
        .map(|l| l.id)
        .collect()
}

#[test]
fn remove_road_makes_vehicles_detour() {
    let mut sim = Simulation::new(detour_world(true));
    let (a, b) = (node(&sim, 0.0, 0.0), node(&sim, 300.0, 0.0));
    let direct = sim
        .world
        .road_graph
        .edges
        .iter()
        .find(|e| e.osm_id == 1)
        .unwrap()
        .id;
    sim.traffic.add_trip(a, b, 5_000).unwrap();
    assert_eq!(sim.traffic.vehicles[0].route, vec![direct * 2]);

    let id = sim.submit(Command::RemoveRoad { edge: direct });
    sim.step(1);
    assert_eq!(sim.outcomes[0].id, id);
    assert_eq!(
        sim.outcomes[0].result,
        CommandResult::Applied {
            edge: Some(direct),
            new_nodes: vec![],
            cells: 0,
            rerouted: 1
        }
    );
    run_to_end(&mut sim);
    let lanes = edge_lanes(&sim, direct);
    assert!(sim.traffic.vehicles[0]
        .trace
        .iter()
        .all(|l| !lanes.contains(l)));
    // 우회로 2구간만 남는다 (없앤 엣지는 ID 유지를 위해 목록에 남지만 통계에서 빠짐)
    assert_eq!(sim.world.road_graph.stats().edges, 2);
    assert_eq!(sim.world.road_graph.edges.len(), 3);
}

#[test]
fn add_road_shortens_route() {
    // 우회로만 있을 때의 도착 시각
    let mut base = Simulation::new(detour_world(false));
    let (a, b) = (node(&base, 0.0, 0.0), node(&base, 300.0, 0.0));
    base.traffic.add_trip(a, b, 30_000).unwrap();
    run_to_end(&mut base);
    let slow = base.traffic.vehicles[0].arrive_ms.unwrap();

    // 출발 전에 A → B 직선 도로를 놓으면 그 길로 간다. 좌표 끝은 5 m 안의 노드에 붙는다.
    let mut sim = Simulation::new(detour_world(false));
    sim.traffic.add_trip(a, b, 30_000).unwrap();
    sim.submit(Command::AddRoad {
        from: RoadEnd::Point(Point { x: 2.0, y: 1.0 }),
        to: RoadEnd::Node(b),
        highway: "residential".into(),
        lanes: Some(2),
        oneway: true,
    });
    sim.step(1);
    let CommandResult::Applied {
        edge: Some(edge),
        ref new_nodes,
        rerouted,
        ..
    } = sim.outcomes[0].result
    else {
        panic!("적용돼야 함: {:?}", sim.outcomes[0]);
    };
    assert!(new_nodes.is_empty());
    assert_eq!(rerouted, 1);
    assert_eq!(sim.world.road_graph.edges[edge as usize].from, a);
    run_to_end(&mut sim);
    let v = &sim.traffic.vehicles[0];
    assert!(v.arrive_ms.unwrap() < slow);
    assert!(v.trace.iter().all(|l| edge_lanes(&sim, edge).contains(l)));
}

#[test]
fn add_road_to_new_point_creates_node() {
    let mut sim = Simulation::new(detour_world(false));
    let before = sim.world.road_graph.nodes.len();
    sim.submit(Command::AddRoad {
        from: RoadEnd::Point(Point { x: 300.0, y: 0.0 }),
        to: RoadEnd::Point(Point { x: 600.0, y: 0.0 }),
        highway: "tertiary".into(),
        lanes: None,
        oneway: false,
    });
    sim.step(1);
    let CommandResult::Applied { ref new_nodes, .. } = sim.outcomes[0].result else {
        panic!("{:?}", sim.outcomes[0]);
    };
    assert_eq!(new_nodes, &vec![before as u32]);
    let far = node(&sim, 600.0, 0.0);
    let b = node(&sim, 300.0, 0.0);
    // 새 양방향 도로로 B ↔ 새 노드를 오갈 수 있다
    assert!(sim.traffic.add_trip(b, far, 0).is_some());
    assert!(sim.traffic.add_trip(far, b, 0).is_some());
    run_to_end(&mut sim);
}

#[test]
fn invalid_commands_are_rejected_without_changes() {
    let mut sim = Simulation::new(detour_world(true));
    let graph_before = sim.world.road_graph.clone();
    let a = node(&sim, 0.0, 0.0);
    let bad = vec![
        Command::RemoveRoad { edge: 999 },
        Command::AddRoad {
            from: RoadEnd::Node(a),
            to: RoadEnd::Point(Point { x: 50.0, y: 0.0 }),
            highway: "footway".into(),
            lanes: None,
            oneway: false,
        },
        Command::AddRoad {
            from: RoadEnd::Node(a),
            to: RoadEnd::Point(Point { x: 50.0, y: 0.0 }),
            highway: "residential".into(),
            lanes: Some(0),
            oneway: false,
        },
        Command::AddRoad {
            from: RoadEnd::Node(a),
            to: RoadEnd::Point(Point { x: 5_000.0, y: 0.0 }),
            highway: "residential".into(),
            lanes: None,
            oneway: false,
        },
        Command::AddRoad {
            from: RoadEnd::Node(9_999),
            to: RoadEnd::Node(a),
            highway: "residential".into(),
            lanes: None,
            oneway: false,
        },
        Command::AddRoad {
            from: RoadEnd::Node(a),
            to: RoadEnd::Point(Point { x: 3.0, y: 0.0 }), // 5 m 안 → 같은 노드
            highway: "residential".into(),
            lanes: None,
            oneway: false,
        },
        Command::SetZone {
            area: ZoneArea::Cells(vec![[0, 0], [9_999, 0]]),
            zone: Some(Zone::Park),
        },
        Command::SetZone {
            area: ZoneArea::Polygon(vec![Point { x: 0.0, y: 0.0 }, Point { x: 1.0, y: 1.0 }]),
            zone: Some(Zone::Park),
        },
    ];
    let n = bad.len();
    for c in bad {
        sim.submit(c);
    }
    sim.step(1);
    assert_eq!(sim.outcomes.len(), n);
    for o in &sim.outcomes {
        assert!(
            matches!(o.result, CommandResult::Rejected { .. }),
            "거부돼야 함: {o:?}"
        );
    }
    assert_eq!(sim.world.road_graph, graph_before);
    assert_eq!(sim.world.zones.zoned().count(), 0);

    // 같은 엣지를 두 번 없애면 두 번째는 거부
    sim.submit(Command::RemoveRoad { edge: 0 });
    sim.submit(Command::RemoveRoad { edge: 0 });
    sim.step(1);
    assert!(matches!(
        sim.outcomes[n].result,
        CommandResult::Applied { .. }
    ));
    assert!(matches!(
        sim.outcomes[n + 1].result,
        CommandResult::Rejected { .. }
    ));
}

#[test]
fn set_zone_cells_polygon_and_clear() {
    let mut sim = Simulation::new(detour_world(true));
    sim.submit(Command::SetZone {
        area: ZoneArea::Cells(vec![[0, 0], [0, 1], [0, 1]]),
        zone: Some(Zone::Residential),
    });
    // 원점 주변 60 m × 60 m 사각형 → 셀 중심 4개
    sim.submit(Command::SetZone {
        area: ZoneArea::Polygon(vec![
            Point { x: -30.0, y: -30.0 },
            Point { x: 30.0, y: -30.0 },
            Point { x: 30.0, y: 30.0 },
            Point { x: -30.0, y: 30.0 },
        ]),
        zone: Some(Zone::Office),
    });
    sim.submit(Command::SetZone {
        area: ZoneArea::Cells(vec![[0, 1]]),
        zone: None,
    });
    assert_eq!(
        sim.world.zones.zoned().count(),
        0,
        "다음 틱 전에는 적용 안 됨"
    );
    sim.step(1);
    let cells: Vec<u32> = sim
        .outcomes
        .iter()
        .map(|o| match o.result {
            CommandResult::Applied { cells, .. } => cells,
            _ => panic!("{o:?}"),
        })
        .collect();
    assert_eq!(cells, vec![2, 4, 1]);
    let z = &sim.world.zones;
    assert_eq!(z.get(0, 0), Some(Zone::Residential));
    assert_eq!(z.get(0, 1), None);
    let (r, c) = z.cell_at(-15.0, 15.0).unwrap();
    assert_eq!(z.get(r, c), Some(Zone::Office));
    assert_eq!(z.zoned().count(), 5);
}

fn pyeongtaek() -> Simulation {
    let dir = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../../maps/pyeongtaek-station");
    Simulation::new(load_map(&dir).unwrap())
}

/// 평택역 맵: 100대가 가장 많이 지나는 엣지를 출발 전에 없애고 남쪽 구역을 칠한다.
fn scenario() -> Simulation {
    let mut sim = pyeongtaek();
    assert_eq!(sim.add_random_trips(100, 7, 300_000), 100);
    let mut uses = std::collections::BTreeMap::new();
    for v in &sim.traffic.vehicles {
        for &arc in &v.route {
            *uses.entry(arc / 2).or_insert(0u32) += 1;
        }
    }
    let (&busiest, _) = uses
        .iter()
        .max_by_key(|(e, n)| (**n, std::cmp::Reverse(**e)))
        .unwrap();
    sim.submit(Command::RemoveRoad { edge: busiest });
    sim.submit(Command::SetZone {
        area: ZoneArea::Polygon(vec![
            Point {
                x: -300.0,
                y: -700.0,
            },
            Point {
                x: 300.0,
                y: -700.0,
            },
            Point {
                x: 300.0,
                y: -400.0,
            },
            Point {
                x: -300.0,
                y: -400.0,
            },
        ]),
        zone: Some(Zone::Residential),
    });
    while sim.traffic.stats().arrived < 100 && sim.tick() * TICK_MS < 3 * 3_600_000 {
        sim.step(600);
    }
    sim
}

#[test]
fn pyeongtaek_commands_are_deterministic() {
    let a = scenario();
    let s = a.traffic.stats();
    println!("{:?}\n{s:#?}", a.outcomes);
    assert_eq!(s.arrived, 100, "도로를 없애도 모두 도착");
    let CommandResult::Applied { rerouted, .. } = a.outcomes[0].result else {
        panic!("{:?}", a.outcomes[0]);
    };
    assert!(rerouted > 0);

    let b = scenario();
    assert_eq!(a.outcomes, b.outcomes);
    assert_eq!(a.traffic.vehicles, b.traffic.vehicles);
    assert_eq!(a.world.zones, b.world.zones);
    assert_eq!(a.world.road_graph, b.world.road_graph);
}
