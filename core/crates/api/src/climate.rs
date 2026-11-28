//! 기후 시나리오 파일 (2단계 #38). 형식: schema/climate/README.md, 예시: schema/climate/examples/
//!
//! 동네 정책은 시나리오를 바꾸지 못한다 (ADR 0003). 실제 값(기상청 자료·SSP)을 넣기 전까지는
//! 값을 임의로 정한 `synthetic` 시나리오를 쓰고, 파일의 `source`에 그 사실을 적는다.

use serde::{Deserialize, Serialize};

/// 계절 번호 (ADR 0007: 1년 = 4계절). 0 봄, 1 여름, 2 가을, 3 겨울.
pub const SUMMER: u32 = 1;

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ClimateScenario {
    pub format_version: u32,
    pub name: String,
    /// 출처 설명. synthetic이면 "값을 임의로 정함"을 반드시 적는다
    pub source: String,
    /// SSP 경로 (예: "SSP2-4.5"), synthetic이면 null
    #[serde(default)]
    pub ssp: Option<String>,
    pub years: Vec<ClimateYear>,
    /// 특정 날의 사건 (집중호우·폭염). 계절 진행 때 대표일과 따로 돌린다
    #[serde(default)]
    pub events: Vec<ClimateEvent>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ClimateYear {
    pub year: u32,
    /// 여름 일 최고기온 분포 (°C)
    pub summer_tmax_mean_c: f64,
    pub summer_tmax_sd_c: f64,
    /// 폭염일수 (일 최고기온 33 °C 이상인 날, 연)
    pub heatwave_days: u32,
    /// 1시간 최대 강수량 (mm, 연)
    pub max_1h_rain_mm: f64,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ClimateEvent {
    pub name: String,
    pub year: u32,
    /// 계절 번호 (0 봄, 1 여름, 2 가을, 3 겨울)
    pub season: u32,
    /// 시간별 강수량 0시~23시 (mm/h), 24개. 강우가 없는 사건이면 빈 배열
    #[serde(default)]
    pub rain_mm_per_h: Vec<f64>,
    /// 그날 최고기온 (°C), 없으면 그해 여름 평균
    #[serde(default)]
    pub tmax_c: Option<f64>,
    /// 사건 자료의 출처 (실측이면 관측소·기관, 아니면 "가정")
    pub source: String,
}

/// 0 이상인 유한한 수 (NaN·음수·무한대는 false).
fn nonneg(x: f64) -> bool {
    x.is_finite() && x >= 0.0
}

impl ClimateScenario {
    /// 형식 검증. 잘못되면 이유를 돌려준다.
    pub fn validate(&self) -> Result<(), String> {
        if self.format_version != 0 {
            return Err(format!(
                "지원하지 않는 format_version {}",
                self.format_version
            ));
        }
        if self.source.trim().is_empty() {
            return Err("source(출처)가 비어 있음".into());
        }
        if self.years.is_empty() {
            return Err("years가 비어 있음".into());
        }
        let mut prev = 0;
        for y in &self.years {
            if y.year <= prev {
                return Err(format!("years는 연도 오름차순이어야 함 ({})", y.year));
            }
            prev = y.year;
            if !nonneg(y.summer_tmax_sd_c) || !nonneg(y.max_1h_rain_mm) || y.heatwave_days > 366 {
                return Err(format!("{}년 값이 범위 밖", y.year));
            }
        }
        for e in &self.events {
            if e.season > 3 {
                return Err(format!(
                    "사건 '{}'의 계절 번호 {}가 0~3 밖",
                    e.name, e.season
                ));
            }
            if !e.rain_mm_per_h.is_empty() && e.rain_mm_per_h.len() != 24 {
                return Err(format!("사건 '{}'의 시간별 강수량은 24개여야 함", e.name));
            }
            if e.rain_mm_per_h.iter().any(|&r| !nonneg(r)) {
                return Err(format!("사건 '{}'에 음수·NaN 강수량", e.name));
            }
            if e.source.trim().is_empty() {
                return Err(format!("사건 '{}'의 source가 비어 있음", e.name));
            }
        }
        Ok(())
    }

    /// 해당 연도 값 (없으면 가장 가까운 앞 연도, 그것도 없으면 첫 연도).
    pub fn year(&self, year: u32) -> &ClimateYear {
        self.years
            .iter()
            .rev()
            .find(|y| y.year <= year)
            .unwrap_or(&self.years[0])
    }

    /// 해당 연도·계절의 사건들.
    pub fn events_in(&self, year: u32, season: u32) -> impl Iterator<Item = &ClimateEvent> {
        self.events
            .iter()
            .filter(move |e| e.year == year && e.season == season)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn example() -> ClimateScenario {
        serde_json::from_str(include_str!(
            "../../../../schema/climate/examples/synthetic.json"
        ))
        .unwrap()
    }

    #[test]
    fn synthetic_example_is_valid_and_round_trips() {
        let s = example();
        s.validate().unwrap();
        assert!(s.ssp.is_none() && s.source.contains("임의"));
        let back: ClimateScenario =
            serde_json::from_str(&serde_json::to_string(&s).unwrap()).unwrap();
        assert_eq!(s, back);
        assert_eq!(s.year(2026).year, 2026);
        assert_eq!(s.year(2100).year, s.years.last().unwrap().year);
        let summer: Vec<_> = s.events_in(2026, SUMMER).collect();
        assert_eq!(summer.len(), 2, "집중호우 + 폭염일");
        let storm: Vec<_> = summer
            .iter()
            .filter(|e| !e.rain_mm_per_h.is_empty())
            .collect();
        assert_eq!(storm.len(), 1);
        assert_eq!(storm[0].rain_mm_per_h.len(), 24);
        assert_eq!(s.events_in(2026, 0).count(), 0);
    }

    #[test]
    fn invalid_scenarios_are_rejected() {
        let mut s = example();
        s.events[0].rain_mm_per_h.pop();
        assert!(s.validate().is_err());
        let mut s = example();
        s.events[0].season = 4;
        assert!(s.validate().is_err());
        let mut s = example();
        s.source = " ".into();
        assert!(s.validate().is_err());
        let mut s = example();
        s.years.swap(0, 1);
        assert!(s.validate().is_err());
    }
}
