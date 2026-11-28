//! 하루·계절 진행 (ADR 0007): 경계 기록 순서, 대표일 집계, 시계 건너뛰기, 결정론.

use std::path::PathBuf;

use dongne_api::{Command, Point, Zone, ZoneArea};
use dongne_sim_core::demand::{DemandParams, DAY_MS, HOUR_MS};
use dongne_sim_core::season::{CalendarEvent, DAYS_PER_SEASON};
use dongne_sim_core::{Simulation, TICK_MS};
use dongne_world::load_map;

fn rect(x0: f64, y0: f64, x1: f64, y1: f64) -> ZoneArea {
    ZoneArea::Polygon(vec![
        Point { x: x0, y: y0 },
        Point { x: x1, y: y0 },
        Point { x: x1, y: y1 },
        Point { x: x0, y: y1 },
    ])
}

/// 평택역 맵 + 시나리오 구역 (통근자를 줄여 테스트를 빠르게)
fn sim() -> Simulation {
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
    s
}

fn params() -> DemandParams {
    DemandParams {
        residents_per_cell: 3,
        ..DemandParams::default()
    }
}

#[test]
fn run_day_from_midday_stops_at_midnight() {
    let mut s = sim();
    s.step(12 * HOUR_MS / TICK_MS);
    let r = s.run_day();
    assert_eq!(r.day, 0);
    assert_eq!(s.now_ms(), DAY_MS);
    assert_eq!(s.day(), 1);
    assert_eq!(
        s.calendar.log,
        vec![
            CalendarEvent::DayStart { day: 0 },
            CalendarEvent::DayEnd { day: 0 }
        ]
    );
    assert_eq!(r.trips, 0);
}

#[test]
fn seasons_log_aggregate_and_skip_clock() {
    let mut s = sim();
    let a = s.run_season(&params(), 11);
    let b = s.run_season(&params(), 11);

    assert_eq!(s.now_ms(), 2 * DAYS_PER_SEASON as u64 * DAY_MS);
    assert_eq!(
        s.calendar.log,
        vec![
            CalendarEvent::SeasonStart { season: 0 },
            CalendarEvent::DayStart { day: 0 },
            CalendarEvent::DayEnd { day: 0 },
            CalendarEvent::SeasonEnd { season: 0 },
            CalendarEvent::SeasonStart { season: 1 },
            CalendarEvent::DayStart { day: 90 },
            CalendarEvent::DayEnd { day: 90 },
            CalendarEvent::SeasonEnd { season: 1 },
        ]
    );
    for (i, r) in [&a, &b].into_iter().enumerate() {
        assert_eq!(r.season, i as u32);
        assert_eq!(r.first_day, i as u32 * DAYS_PER_SEASON);
        assert_eq!(r.representative.day, r.first_day);
        assert!(r.representative.trips > 0);
        assert_eq!(r.representative.unfinished, 0, "대표일 안에 모두 도착");
        assert_eq!(r.total_trips, r.representative.trips as u64 * 90);
        assert!(r.mean_commute_s > 0.0);
        // 출퇴근 피크: 08시·18시대가 03시보다 붐빔
        let h = r.representative.on_road_by_hour;
        assert!(h[8] > h[3] && h[18] > h[3]);
    }
    // 계절마다 시드가 달라 통근자 배정이 다르다 (통행 수는 같음)
    assert_eq!(a.representative.trips, b.representative.trips);
    assert_eq!(s.calendar.seasons, vec![a, b]);
    // 두 계절 사이 날짜에는 통행이 없다
    assert_eq!(s.day_report(45).trips, 0);
}

#[test]
fn seasons_are_deterministic() {
    let run = || {
        let mut s = sim();
        let r = (s.run_season(&params(), 5), s.run_season(&params(), 5));
        (r, s.traffic.vehicles.clone(), s.calendar.clone())
    };
    assert_eq!(run(), run());
}
