//! 하루·계절 진행 (ADR 0007: 틱 100 ms, 하루, 계절 90일, 대표일 방식).
//!
//! - `run_day`: 지금 시각부터 다음 자정까지 진행하고 그날의 보고서를 낸다.
//! - `run_season`: 계절 첫날(대표 평일)의 출퇴근을 차량 단위로 돌리고, 결과를 90일로 집계한 뒤
//!   시계를 다음 계절 첫날 자정으로 넘긴다. 기후 사건일은 2단계 기후 이슈에서 추가한다.
//! - 경계 기록(`Calendar::log`)은 인구·경제·기후 모델이 갱신되는 지점이다.

use crate::demand::{DemandParams, DAY_MS, HOUR_MS};
use crate::{Simulation, TICK_MS};

pub const DAYS_PER_SEASON: u32 = 90;
pub const SEASONS_PER_YEAR: u32 = 4;
pub const DAY_TICKS: u64 = DAY_MS / TICK_MS;

/// 하루·계절 경계 기록.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CalendarEvent {
    SeasonStart { season: u32 },
    DayStart { day: u32 },
    DayEnd { day: u32 },
    SeasonEnd { season: u32 },
}

#[derive(Debug, Clone, Default, PartialEq)]
pub struct Calendar {
    pub log: Vec<CalendarEvent>,
    pub seasons: Vec<SeasonReport>,
    /// 지금까지 쌓인 부채 (원, #37 경제 모델)
    pub debt: f64,
}

/// 하루 보고서: 그날 출발한 통행만 센다.
#[derive(Debug, Clone, PartialEq, Default)]
pub struct DayReport {
    /// 0부터 세는 날짜 (시뮬레이션 시각 ÷ 하루)
    pub day: u32,
    pub trips: u32,
    pub arrived: u32,
    /// 그날 안에 도착하지 못한 통행 (다음 날로 넘어감)
    pub unfinished: u32,
    pub mean_travel_s: f64,
    pub max_travel_s: f64,
    /// 시간대별 도로 위 차량 수 (그 시간 안에 한 번이라도 주행 중)
    pub on_road_by_hour: [u32; 24],
}

/// 계절 보고서 (대표일 × 90일).
#[derive(Debug, Clone, PartialEq, Default)]
pub struct SeasonReport {
    /// 0부터 세는 계절 번호 (0 = 1년차 첫 계절)
    pub season: u32,
    pub first_day: u32,
    pub days: u32,
    pub representative: DayReport,
    /// 계절 전체 통행 수 (대표일 통행 × 일수)
    pub total_trips: u64,
    pub mean_commute_s: f64,
    /// 계절 재정 (#37)
    pub budget: crate::economy::SeasonBudget,
}

impl DayReport {
    pub fn to_json(&self) -> serde_json::Value {
        serde_json::json!({
            "day": self.day, "trips": self.trips, "arrived": self.arrived,
            "unfinished": self.unfinished, "mean_travel_s": self.mean_travel_s,
            "max_travel_s": self.max_travel_s, "on_road_by_hour": self.on_road_by_hour.to_vec(),
        })
    }
}

impl SeasonReport {
    pub fn to_json(&self) -> serde_json::Value {
        serde_json::json!({
            "season": self.season, "first_day": self.first_day, "days": self.days,
            "total_trips": self.total_trips, "mean_commute_s": self.mean_commute_s,
            "representative": self.representative.to_json(),
            "budget": {
                "property_tax": self.budget.property_tax, "road_upkeep": self.budget.road_upkeep,
                "policy_cost": self.budget.policy_cost, "balance": self.budget.balance,
                "debt_after": self.budget.debt_after, "mean_land_value": self.budget.mean_land_value,
            },
        })
    }
}

impl Simulation {
    /// 지금 시뮬레이션 시각 (ms).
    pub fn now_ms(&self) -> u64 {
        self.tick() * TICK_MS
    }

    /// 지금 날짜 (0부터).
    pub fn day(&self) -> u32 {
        (self.now_ms() / DAY_MS) as u32
    }

    /// 다음 자정까지 진행하고 그날의 보고서를 낸다 (자정에 부르면 꼬박 하루).
    pub fn run_day(&mut self) -> DayReport {
        let day = self.day();
        let start = day as u64 * DAY_MS;
        let end = start + DAY_MS;
        self.calendar.log.push(CalendarEvent::DayStart { day });
        // 1시간씩 진행 (명령은 첫 step에서 적용)
        while self.now_ms() < end {
            let left = (end - self.now_ms()) / TICK_MS;
            self.step(left.min(HOUR_MS / TICK_MS));
        }
        self.calendar.log.push(CalendarEvent::DayEnd { day });
        self.day_report(day)
    }

    /// `day`에 출발한 통행의 보고서.
    pub fn day_report(&self, day: u32) -> DayReport {
        let start = day as u64 * DAY_MS;
        let end = start + DAY_MS;
        let mut r = DayReport {
            day,
            ..Default::default()
        };
        let mut sum = 0.0;
        for v in &self.traffic.vehicles {
            if v.depart_ms < start || v.depart_ms >= end {
                continue;
            }
            r.trips += 1;
            match v.arrive_ms {
                Some(a) if a < end => {
                    r.arrived += 1;
                    let t = (a - v.depart_ms) as f64 / 1000.0;
                    sum += t;
                    r.max_travel_s = r.max_travel_s.max(t);
                }
                _ => r.unfinished += 1,
            }
            let until = v.arrive_ms.unwrap_or(u64::MAX).min(end - 1);
            for (h, count) in r.on_road_by_hour.iter_mut().enumerate() {
                let (a, b) = (start + h as u64 * HOUR_MS, start + (h as u64 + 1) * HOUR_MS);
                if v.depart_ms < b && until >= a {
                    *count += 1;
                }
            }
        }
        if r.arrived > 0 {
            r.mean_travel_s = sum / r.arrived as f64;
        }
        r
    }

    /// 한 계절을 진행한다 (ADR 0007 대표일 방식).
    ///
    /// 1. 지금 날짜가 자정이 아니면 다음 자정까지 먼저 진행하고, 쌓인 명령을 적용한다.
    /// 2. 계절 첫날의 출퇴근을 지금 구역 맵으로 만들어(`seed` + 계절 번호) 하루를 차량 단위로 돌린다.
    /// 3. 결과를 `DAYS_PER_SEASON`일로 집계하고, 시계를 다음 계절 첫날 자정으로 넘긴다
    ///    (그 사이에는 예약된 통행이 없으므로 이벤트 없이 건너뛴다).
    pub fn run_season(&mut self, params: &DemandParams, seed: u64) -> SeasonReport {
        if !self.now_ms().is_multiple_of(DAY_MS) {
            self.run_day();
        }
        // 계절 시작 전에 쌓인 명령(구역·도로 변경)을 먼저 적용해 그 상태로 수요를 만든다
        self.step(0);
        let season = self.calendar.seasons.len() as u32;
        let first_day = self.day();
        self.calendar
            .log
            .push(CalendarEvent::SeasonStart { season });

        self.add_commute_at(
            params,
            seed.wrapping_add(season as u64),
            first_day as u64 * DAY_MS,
        );
        let rep = self.run_day();

        // 남은 89일은 차량 단위로 돌리지 않고 건너뛴다
        let next = (first_day + DAYS_PER_SEASON) as u64 * DAY_MS;
        self.step((next - self.now_ms()) / TICK_MS);
        self.calendar.log.push(CalendarEvent::SeasonEnd { season });

        let report = SeasonReport {
            season,
            first_day,
            days: DAYS_PER_SEASON,
            total_trips: rep.trips as u64 * DAYS_PER_SEASON as u64,
            mean_commute_s: rep.mean_travel_s,
            budget: {
                let b = crate::economy::season_budget(
                    &self.world,
                    &crate::economy::EconomyParams::default(),
                    0.0,
                    self.calendar.debt,
                );
                self.calendar.debt = b.debt_after;
                b
            },
            representative: rep,
        };
        self.calendar.seasons.push(report.clone());
        report
    }
}
