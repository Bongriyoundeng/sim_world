//! 건물 → 통행 수요 (#36): 같은 연면적이라도 용도별로 다른 통행량, 결정론, 평택역 맵 규모.

use std::path::PathBuf;

use dongne_sim_core::demand::{DemandParams, DAY_MS, HOUR_MS};
use dongne_sim_core::Simulation;
use dongne_world::buildings::Building;
use dongne_world::land_use::{BuildingUse, LandUseParams};
use dongne_world::{load_map, parse_manifest, RoadGraph, World, ZoneMap};

fn feature(osm_id: i64, pts: &[(f64, f64)]) -> String {
    let coords: Vec<String> = pts.iter().map(|(x, y)| format!("[{x},{y}]")).collect();
    format!(
        r#"{{"type":"Feature","properties":{{"osm_id":{osm_id},"highway":"tertiary","name":null,"lanes":4,"oneway":false}},"geometry":{{"type":"LineString","coordinates":[{}]}}}}"#,
        coords.join(",")
    )
}

fn square(id: i64, kind: &str, x: f64, y: f64, side: f64, levels: f64) -> Building {
    Building {
        osm_id: id,
        kind: kind.into(),
        levels: Some(levels),
        height_m: levels * 3.0,
        footprint: vec![(x, y), (x + side, y), (x + side, y + side), (x, y + side)],
    }
}

/// 동서 도로(0~900 m) 위에 노드 A(0)·B(300)·C(600)·D(900). A 옆 아파트, B 옆 사무, C 옆 상가, D 옆 아파트.
/// 사무·상가는 연면적이 같다(40 m × 40 m × 5층 = 8,000㎡).
fn world() -> World {
    let manifest = parse_manifest(
        r#"{"format_version":0,"name":"t","origin":{"lat":0,"lon":0},"crs":"local-meters",
        "extent_m":{"width":2000,"height":2000},
        "terrain":{"file":"terrain.png","resolution_m":30,"min_elevation_m":0,"max_elevation_m":1},
        "attribution":["test"]}"#,
    )
    .unwrap();
    let json = format!(
        r#"{{"type":"FeatureCollection","features":[{},{},{}]}}"#,
        feature(1, &[(0.0, 0.0), (300.0, 0.0)]),
        feature(2, &[(300.0, 0.0), (600.0, 0.0)]),
        feature(3, &[(600.0, 0.0), (900.0, 0.0)]),
    );
    World {
        road_graph: RoadGraph::from_geojson(&json, 2000.0, 2000.0).unwrap(),
        zones: ZoneMap::new(2000.0, 2000.0, 30.0),
        manifest,
        buildings: vec![
            square(1, "apartments", -20.0, 10.0, 40.0, 15.0),
            square(2, "office", 280.0, 10.0, 40.0, 5.0),
            square(3, "retail", 580.0, 10.0, 40.0, 5.0),
            square(4, "apartments", 880.0, 10.0, 40.0, 15.0),
        ],
    }
}

fn dp() -> DemandParams {
    DemandParams {
        max_access_m: 100.0,
        ..DemandParams::default()
    }
}

#[test]
fn same_floor_area_different_trips_by_use() {
    let mut s = Simulation::new(world());
    let r = s.add_building_demand_at(&LandUseParams::default(), &dp(), 1, 0);
    println!("{r:#?}");
    let office = &r.by_use[&BuildingUse::Office];
    let retail = &r.by_use[&BuildingUse::Commercial];
    let res = &r.by_use[&BuildingUse::Residential];
    assert_eq!(office.floor_area_m2, retail.floor_area_m2);
    // 사무는 종사자가 많아 출퇴근 도착이 많고, 상가는 방문 도착이 많다 → 둘 다 통행을 끌어오지만 양이 다르다
    assert!(office.workers > retail.workers);
    assert_ne!(office.trips_to, retail.trips_to);
    assert!(res.trips_from > 0 && res.residents > 0.0);
    assert_eq!(r.unroutable, 0);

    // 사무 도착은 아침에, 상가 도착은 낮에 몰린다
    s.run_day();
    let (b, c) = (
        s.world.road_graph.nearest_node(300.0, 0.0, 1.0).unwrap(),
        s.world.road_graph.nearest_node(600.0, 0.0, 1.0).unwrap(),
    );
    let mean_hour = |node| {
        let ts: Vec<f64> = s
            .traffic
            .vehicles
            .iter()
            .filter(|v| v.dest == node && v.depart_ms < 12 * HOUR_MS + DAY_MS)
            .map(|v| v.depart_ms as f64 / HOUR_MS as f64)
            .collect();
        ts.iter().sum::<f64>() / ts.len() as f64
    };
    let (office_h, retail_h) = (mean_hour(b), mean_hour(c));
    assert!(office_h < 9.5, "사무 도착 평균 {office_h:.1}시");
    assert!(retail_h > 10.0, "상가 도착 평균 {retail_h:.1}시");
}

#[test]
fn more_floor_area_more_trips_and_deterministic() {
    let run = |levels: f64| {
        let mut w = world();
        w.buildings[1].levels = Some(levels);
        let mut s = Simulation::new(w);
        s.add_building_demand_at(&LandUseParams::default(), &dp(), 7, 0)
    };
    let (small, big) = (run(2.0), run(10.0));
    assert!(
        big.by_use[&BuildingUse::Office].trips_to > small.by_use[&BuildingUse::Office].trips_to,
        "사무 연면적이 크면 도착 통행이 많다"
    );
    assert_eq!(run(5.0), run(5.0));
}

#[test]
fn pyeongtaek_buildings_give_population_and_jobs() {
    let dir = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../../maps/pyeongtaek-station");
    let mut s = Simulation::new(load_map(&dir).unwrap());
    let r = s.add_building_demand_at(&LandUseParams::default(), &DemandParams::default(), 2026, 0);
    println!(
        "거주 {:.0}명, 종사자 {:.0}명, 차 통근 {}명, 방문 {}회, 통행 {}개, 도로 없는 건물 {}",
        r.residents, r.workers, r.car_commuters, r.visit_trips, r.trips, r.buildings_without_road
    );
    for (u, t) in &r.by_use {
        println!(
            "{u:?}: 건물 {} 연면적 {:.0}㎡ 거주 {:.0} 종사 {:.0} 출발 {} 도착 {}",
            t.buildings, t.floor_area_m2, t.residents, t.workers, t.trips_from, t.trips_to
        );
    }
    assert!(r.residents > 1_000.0 && r.workers > 1_000.0 && r.trips > 0);
}
