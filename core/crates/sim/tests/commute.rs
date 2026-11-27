//! 구역 → 통근 수요: 일자리 수 제한, 시간 분포, 평택역 맵의 출퇴근 피크, 결정론.

use std::path::PathBuf;

use dongne_api::{Command, Point, Zone, ZoneArea};
use dongne_sim_core::demand::{
    generate_commuters, vehicles_on_road_by_hour, DemandParams, HOUR_MS,
};
use dongne_sim_core::{Simulation, VehicleState};
use dongne_world::{load_map, parse_manifest, RoadGraph, World, ZoneMap};

fn feature(osm_id: i64, pts: &[(f64, f64)]) -> String {
    let coords: Vec<String> = pts.iter().map(|(x, y)| format!("[{x},{y}]")).collect();
    format!(
        r#"{{"type":"Feature","properties":{{"osm_id":{osm_id},"highway":"residential","name":null,"lanes":2,"oneway":false}},"geometry":{{"type":"LineString","coordinates":[{}]}}}}"#,
        coords.join(",")
    )
}

/// 300 m × 300 m, 30 m 격자 10×10, 가로 도로 하나 (y = 0)
fn small_world() -> World {
    let manifest = parse_manifest(
        r#"{"format_version":0,"name":"t","origin":{"lat":0,"lon":0},"crs":"local-meters",
        "extent_m":{"width":300,"height":300},
        "terrain":{"file":"terrain.png","resolution_m":30,"min_elevation_m":0,"max_elevation_m":1},
        "attribution":["test"]}"#,
    )
    .unwrap();
    let json = format!(
        r#"{{"type":"FeatureCollection","features":[{}]}}"#,
        feature(1, &[(-150.0, 0.0), (0.0, 0.0), (150.0, 0.0)])
    );
    World {
        road_graph: RoadGraph::from_geojson(&json, 300.0, 300.0).unwrap(),
        zones: ZoneMap::new(300.0, 300.0, 30.0),
        manifest,
        buildings: Vec::new(),
    }
}

fn params(residents: u32, office_jobs: u32) -> DemandParams {
    DemandParams {
        residents_per_cell: residents,
        jobs_per_office_cell: office_jobs,
        jobs_per_commercial_cell: 0,
        max_access_m: 100.0,
        ..DemandParams::default()
    }
}

#[test]
fn jobs_limit_commuters_and_far_cells_are_skipped() {
    let mut w = small_world();
    // 서쪽 끝 주거 2셀, 동쪽 끝 업무 1셀 (도로 y=0 근처), 북쪽 모서리 주거 1셀(도로에서 135 m 이상)
    w.zones.set(&[(4, 0), (5, 0)], Some(Zone::Residential));
    w.zones.set(&[(5, 9)], Some(Zone::Office));
    w.zones.set(&[(0, 0)], Some(Zone::Residential));

    let (cs, s) = generate_commuters(&w.zones, &w.road_graph, &params(10, 30), 1);
    assert_eq!(s.cells_without_road, 1);
    assert_eq!((s.residents, s.jobs, s.commuters), (20, 30, 20));
    assert!(cs.iter().all(|c| c.work_cell == (5, 9)));
    assert!(cs.iter().all(|c| c.leave_work_ms > c.leave_home_ms));

    // 일자리가 5개뿐이면 5명만 통근
    let (cs, s) = generate_commuters(&w.zones, &w.road_graph, &params(10, 5), 1);
    assert_eq!((s.commuters, cs.len()), (5, 5));
}

#[test]
fn departure_times_center_on_peaks() {
    let mut w = small_world();
    w.zones.set(&[(5, 0)], Some(Zone::Residential));
    w.zones.set(&[(5, 9)], Some(Zone::Office));
    let (cs, _) = generate_commuters(&w.zones, &w.road_graph, &params(4_000, 4_000), 3);
    let mean = |f: fn(&dongne_sim_core::Commuter) -> u64| {
        cs.iter().map(|c| f(c) as f64).sum::<f64>() / cs.len() as f64 / HOUR_MS as f64
    };
    let (m, e) = (mean(|c| c.leave_home_ms), mean(|c| c.leave_work_ms));
    assert!((m - 8.0).abs() < 0.05, "출근 평균 {m}시");
    assert!((e - 18.0).abs() < 0.05, "퇴근 평균 {e}시");
    // 표준편차 30분 → 약 95%가 평균 ±1시간 안
    let within = cs
        .iter()
        .filter(|c| c.leave_home_ms.abs_diff(8 * HOUR_MS) <= HOUR_MS)
        .count() as f64
        / cs.len() as f64;
    assert!(within > 0.93 && within < 0.98, "{within}");
}

fn rect(x0: f64, y0: f64, x1: f64, y1: f64) -> ZoneArea {
    ZoneArea::Polygon(vec![
        Point { x: x0, y: y0 },
        Point { x: x1, y: y0 },
        Point { x: x1, y: y1 },
        Point { x: x0, y: y1 },
    ])
}

/// 평택역 맵: 남서쪽 주거, 역 앞 업무, 동쪽 상업을 칠하고 하루를 돌린다.
fn pyeongtaek_day() -> (Simulation, dongne_sim_core::CommuteReport) {
    let dir = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../../maps/pyeongtaek-station");
    let mut sim = Simulation::new(load_map(&dir).unwrap());
    for (area, zone) in [
        (rect(-700.0, -700.0, -300.0, -300.0), Zone::Residential),
        (rect(-250.0, -400.0, 50.0, -100.0), Zone::Office),
        (rect(200.0, 0.0, 500.0, 300.0), Zone::Commercial),
    ] {
        sim.submit(Command::SetZone {
            area,
            zone: Some(zone),
        });
    }
    sim.step(1);
    let report = sim.add_commute(&DemandParams::default(), 2026);
    while sim.tick() * dongne_sim_core::TICK_MS < 24 * HOUR_MS {
        sim.step(36_000); // 1시간씩
    }
    (sim, report)
}

#[test]
fn pyeongtaek_commute_has_morning_and_evening_peaks() {
    let (sim, report) = pyeongtaek_day();
    let stats = sim.traffic.stats();
    let hours = vehicles_on_road_by_hour(&sim.traffic.vehicles);
    println!("{report:#?}\n{stats:#?}\n시간대별 도로 위 차량: {hours:?}");

    assert!(report.demand.commuters > 0 && report.unroutable == 0);
    assert_eq!(
        stats.arrived, stats.vehicles,
        "하루 안에 모든 통행이 끝나야 함"
    );
    assert!(sim
        .traffic
        .vehicles
        .iter()
        .all(|v| v.state == VehicleState::Arrived));

    let morning_peak = (5..12)
        .max_by_key(|&h| (hours[h], std::cmp::Reverse(h)))
        .unwrap();
    let evening_peak = (14..22)
        .max_by_key(|&h| (hours[h], std::cmp::Reverse(h)))
        .unwrap();
    assert!(
        (7..=8).contains(&morning_peak),
        "출근 피크 {morning_peak}시"
    );
    assert!(
        (17..=18).contains(&evening_peak),
        "퇴근 피크 {evening_peak}시"
    );
    // 피크 시간대가 한밤·한낮보다 훨씬 붐빈다
    for quiet in [3, 12] {
        assert!(
            hours[quiet] * 10 < hours[morning_peak],
            "{quiet}시 {}",
            hours[quiet]
        );
        assert!(
            hours[quiet] * 10 < hours[evening_peak],
            "{quiet}시 {}",
            hours[quiet]
        );
    }

    let (again, report2) = pyeongtaek_day();
    assert_eq!(report, report2);
    assert_eq!(sim.traffic.vehicles, again.traffic.vehicles);
}
