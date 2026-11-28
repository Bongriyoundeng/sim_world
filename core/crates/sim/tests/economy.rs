//! 경제 모델 (#37): 밀도 → 승용차 분담률, 토지가치(접근성·녹지), 재산세·예산, 도로 추가 효과.

use dongne_api::{Command, Point, RoadEnd, Zone, ZoneArea};
use dongne_sim_core::demand::DemandParams;
use dongne_sim_core::economy::{car_share_for_density, land_values, season_budget, EconomyParams};
use dongne_sim_core::Simulation;
use dongne_world::buildings::Building;
use dongne_world::land_use::LandUseParams;
use dongne_world::{parse_manifest, RoadGraph, World, ZoneMap};

fn feature(osm_id: i64, highway: &str, pts: &[(f64, f64)]) -> String {
    let coords: Vec<String> = pts.iter().map(|(x, y)| format!("[{x},{y}]")).collect();
    format!(
        r#"{{"type":"Feature","properties":{{"osm_id":{osm_id},"highway":"{highway}","name":null,"lanes":2,"oneway":false}},"geometry":{{"type":"LineString","coordinates":[{}]}}}}"#,
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

/// 600 m × 600 m. 동서 간선(tertiary) y=0, 이면도로(residential) y=-240.
/// 아파트(서쪽, 층수 `levels`)와 사무(동쪽).
fn world(levels: f64) -> World {
    let manifest = parse_manifest(
        r#"{"format_version":0,"name":"t","origin":{"lat":0,"lon":0},"crs":"local-meters",
        "extent_m":{"width":600,"height":600},
        "terrain":{"file":"terrain.png","resolution_m":30,"min_elevation_m":0,"max_elevation_m":1},
        "attribution":["test"]}"#,
    )
    .unwrap();
    let json = format!(
        r#"{{"type":"FeatureCollection","features":[{},{},{},{}]}}"#,
        feature(1, "tertiary", &[(-300.0, 0.0), (0.0, 0.0)]),
        feature(2, "tertiary", &[(0.0, 0.0), (300.0, 0.0)]),
        feature(3, "residential", &[(-300.0, -240.0), (300.0, -240.0)]),
        feature(4, "residential", &[(-300.0, 0.0), (-300.0, -240.0)]),
    );
    World {
        road_graph: RoadGraph::from_geojson(&json, 600.0, 600.0).unwrap(),
        zones: ZoneMap::new(600.0, 600.0, 30.0),
        manifest,
        buildings: vec![
            square(1, "apartments", -290.0, 10.0, 30.0, levels),
            square(2, "office", 260.0, 10.0, 30.0, 10.0),
        ],
    }
}

#[test]
fn car_share_falls_with_density() {
    let p = EconomyParams::default();
    let shares: Vec<f64> = [0.0, 5_000.0, 15_000.0, 50_000.0, 1e7]
        .iter()
        .map(|&d| car_share_for_density(d, &p))
        .collect();
    assert_eq!(shares[0], p.car_share_max);
    assert!(shares.windows(2).all(|w| w[1] < w[0]));
    assert!((shares[2] - (p.car_share_min + p.car_share_max) / 2.0).abs() < 1e-12);
    assert!(shares[4] - p.car_share_min < 0.01);

    // 같은 아파트라도 층수가 높아 밀도가 크면 차 통근 비율이 낮다
    let dp = DemandParams {
        max_access_m: 100.0,
        ..DemandParams::default()
    };
    let run = |levels| {
        let mut s = Simulation::new(world(levels));
        s.add_building_demand_at(&LandUseParams::default(), &dp, 1, 0)
    };
    let (low, high) = (run(2.0), run(30.0));
    assert!(high.mean_car_share < low.mean_car_share);
}

#[test]
fn land_value_rises_near_major_roads_and_parks() {
    let mut w = world(5.0);
    let p = EconomyParams::default();
    let lv = land_values(&w, &p);
    let z = w.zones.clone();
    let at = |lv: &Vec<f64>, x: f64, y: f64| {
        let (r, c) = z.cell_at(x, y).unwrap();
        lv[(r * z.cols + c) as usize]
    };
    let near = at(&lv, 0.0, 10.0);
    let far = at(&lv, 0.0, -280.0);
    assert!(near > far, "간선 옆이 더 비쌈: {near} vs {far}");

    let cell = z.cell_at(0.0, -280.0).unwrap();
    w.zones.set(&[cell], Some(Zone::Park));
    let with_park = at(&land_values(&w, &p), 0.0, -280.0);
    assert!((with_park / far - (1.0 + p.green_weight)).abs() < 1e-9);
}

#[test]
fn new_major_road_raises_nearby_land_value() {
    let p = EconomyParams::default();
    let mut s = Simulation::new(world(5.0));
    let before = land_values(&s.world, &p);
    s.submit(Command::AddRoad {
        from: RoadEnd::Point(Point {
            x: -300.0,
            y: -240.0,
        }),
        to: RoadEnd::Point(Point {
            x: 300.0,
            y: -240.0,
        }),
        highway: "tertiary".into(),
        lanes: Some(2),
        oneway: false,
    });
    s.step(0);
    let after = land_values(&s.world, &p);
    let z = &s.world.zones;
    let (r, c) = z.cell_at(0.0, -255.0).unwrap();
    let i = (r * z.cols + c) as usize;
    assert!(after[i] > before[i]);
    assert!(after.iter().zip(&before).all(|(a, b)| a >= b));
}

#[test]
fn budget_responds_to_tax_rate_and_roads() {
    let w = world(5.0);
    let p = EconomyParams::default();
    let base = season_budget(&w, &p, 0.0, 0.0);
    assert!(base.property_tax > 0.0 && base.road_upkeep > 0.0);
    let double = season_budget(
        &w,
        &EconomyParams {
            property_tax_rate: p.property_tax_rate * 2.0,
            ..p.clone()
        },
        0.0,
        0.0,
    );
    assert!((double.property_tax / base.property_tax - 2.0).abs() < 1e-9);
    // 정책 비용이 세수보다 크면 적자가 부채로 쌓인다
    let costly = season_budget(&w, &p, base.property_tax * 10.0, 100.0);
    assert!(costly.balance < 0.0 && costly.debt_after > 100.0);
    let _ = ZoneArea::Cells(vec![]);
}
