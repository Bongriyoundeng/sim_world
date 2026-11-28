//! 플랫폼 간 결정론 (이슈 #33): 고정 시나리오의 상태 해시가 기록해 둔 값과 같아야 한다.
//!
//! CI는 이 테스트를 Linux와 Windows에서 모두 돌린다. 한쪽만 실패하면 플랫폼마다 계산이 다른 것이다.
//! 시뮬레이션 동작을 일부러 바꿨다면 `GOLDEN`을 새 값으로 바꾸고 PR에 이유를 적는다
//! (`cargo test -p dongne-sim-core --test determinism -- --nocapture`가 새 값을 출력한다).

use std::path::PathBuf;

use dongne_api::{Command, Point, RoadEnd, Zone, ZoneArea};
use dongne_sim_core::demand::{DemandParams, HOUR_MS};
use dongne_sim_core::{Simulation, TICK_MS};
use dongne_world::load_map;

/// 고정 시나리오의 기대 상태 해시
const GOLDEN: &str = "144293afcf745f69";

fn rect(x0: f64, y0: f64, x1: f64, y1: f64) -> ZoneArea {
    ZoneArea::Polygon(vec![
        Point { x: x0, y: y0 },
        Point { x: x1, y: y0 },
        Point { x: x1, y: y1 },
        Point { x: x0, y: y1 },
    ])
}

/// 구역 → 계절 1개 → 도로 추가·삭제 → 무작위 통행 → 하루
fn scenario() -> Simulation {
    let dir = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../../maps/pyeongtaek-station");
    let mut s = Simulation::new(load_map(&dir).unwrap());
    for (area, zone) in [
        (rect(-700.0, -700.0, -300.0, -300.0), Zone::Residential),
        (rect(-250.0, -400.0, 50.0, -100.0), Zone::Office),
        (rect(200.0, 0.0, 500.0, 300.0), Zone::Commercial),
    ] {
        s.submit(Command::SetZone {
            area,
            zone: Some(zone),
        });
    }
    let p = DemandParams {
        residents_per_cell: 3,
        ..DemandParams::default()
    };
    s.run_season(&p, 1);

    s.submit(Command::AddRoad {
        from: RoadEnd::Point(Point {
            x: -600.0,
            y: 600.0,
        }),
        to: RoadEnd::Point(Point {
            x: -400.0,
            y: 650.0,
        }),
        highway: "residential".into(),
        lanes: Some(2),
        oneway: false,
    });
    let busiest = s.edge_usage()[0].0;
    s.submit(Command::RemoveRoad { edge: busiest });
    s.step(1);
    // 출발 시각(0~1시간)이 이미 지났으므로 모두 지금 바로 출발한다 (도로를 바꾼 직후의 교통)
    s.add_random_trips(80, 9, HOUR_MS);
    s.step(8 * HOUR_MS / TICK_MS);
    s.run_day();
    s
}

#[test]
fn state_hash_is_stable_and_sensitive() {
    let a = scenario();
    let b = scenario();
    assert_eq!(a.state_hash(), b.state_hash(), "같은 입력이면 같은 해시");

    let mut c = scenario();
    c.submit(Command::SetZone {
        area: ZoneArea::Cells(vec![[0, 0]]),
        zone: Some(Zone::Park),
    });
    c.step(0);
    assert_ne!(a.state_hash(), c.state_hash(), "상태가 다르면 해시도 다름");
}

#[test]
fn state_hash_matches_golden_on_every_platform() {
    let h = scenario().state_hash_hex();
    println!("state_hash = {h}");
    assert_eq!(
        h, GOLDEN,
        "플랫폼 간 결정론: 기록한 해시와 다름 (일부러 바꿨다면 GOLDEN 갱신)"
    );
}
