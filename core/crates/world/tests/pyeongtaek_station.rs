//! 저장소의 샘플 맵 패키지 maps/pyeongtaek-station을 읽는 통합 테스트.

use std::path::PathBuf;

use dongne_world::load_map;

fn map_dir() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../../maps/pyeongtaek-station")
}

#[test]
fn loads_pyeongtaek_station_road_graph() {
    let world = load_map(&map_dir()).expect("샘플 맵 패키지를 읽을 수 있어야 함");
    let g = &world.road_graph;
    let s = g.stats();
    println!("{s:#?}");
    assert!(s.nodes > 0 && s.edges > 0 && s.lanes > 0);
    assert!(
        s.boundary_nodes > 0,
        "1.5km 구역 경계에서 잘린 도로가 있어야 함"
    );
    assert!(s.excluded.contains_key("footway"));
    // 모든 엣지는 존재하는 노드를 잇고, 길이가 양수
    for e in &g.edges {
        assert!((e.from as usize) < g.nodes.len() && (e.to as usize) < g.nodes.len());
        assert!(e.length_m > 0.0);
        assert!(e.lanes_forward + e.lanes_backward > 0);
    }
    assert_eq!(world.buildings.len(), 3048);
    assert!(world
        .buildings
        .iter()
        .all(|b| b.height_m > 0.0 && b.footprint.len() >= 3));
    assert!(
        s.largest_scc_ratio >= MIN_SCC_RATIO,
        "최대 강연결 요소 비율 {}",
        s.largest_scc_ratio
    );
}

/// 최대 강연결 요소 비율 하한. 2026-11-27 OSM 기준 측정값은 0.943 (core/README.md).
/// OSM이 갱신돼 맵을 다시 만들어도 깨지지 않게 여유를 둔다.
const MIN_SCC_RATIO: f64 = 0.8;

#[test]
fn road_graph_is_deterministic() {
    let a = load_map(&map_dir()).unwrap().road_graph;
    let b = load_map(&map_dir()).unwrap().road_graph;
    assert_eq!(a, b);
}
