//! 경제 모델 (2단계 #37): 셀 토지가치, 재산세·예산, 밀도 → 승용차 분담률.
//!
//! - 토지가치(원/㎡) = 기준값 × (1 + 접근성 가중 × 접근성) × (1 + 녹지 가중 × 공원·녹지 비율)
//!   접근성 = 1 / (1 + 가장 가까운 간선(tertiary 이상) 도로 선분까지 거리 / 300 m)
//!   기준값은 개별공시지가(ADR 0008, VWorld 제공)를 받으면 셀 평균으로 바꾼다. 그 전에는 가정값.
//! - 재산세(계절) = Σ 셀 토지가치 × 셀 면적 × 세율 ÷ 4
//! - 예산(계절) = 재산세 − 도로 유지비(차선 km당) − 정책 비용, 적자는 부채로 쌓임
//! - 승용차 분담률 = 최소 + (최대 − 최소) ÷ (1 + 거주 밀도 ÷ 반감 밀도) — 밀도가 높을수록 낮아짐
//!
//! 모든 계수는 가정이다: docs/design/research-log.md#economy

use dongne_world::road_graph::norm;
use dongne_world::{World, Zone};

use crate::traffic::is_major_road;

#[derive(Debug, Clone, PartialEq)]
pub struct EconomyParams {
    /// 기준 토지가치 (원/㎡, 가정 — 개별공시지가로 교체 예정)
    pub base_land_value: f64,
    pub access_weight: f64,
    pub access_distance_m: f64,
    pub green_weight: f64,
    /// 토지 재산세 실효 세율 (연, 가정)
    pub property_tax_rate: f64,
    /// 차선 1 km 계절 유지비 (원, 가정)
    pub road_upkeep_per_lane_km: f64,
    /// 승용차 분담률 범위와 반감 밀도 (명/㎢, 가정)
    pub car_share_max: f64,
    pub car_share_min: f64,
    pub car_share_half_density: f64,
}

impl Default for EconomyParams {
    fn default() -> Self {
        Self {
            base_land_value: 1_000_000.0,
            access_weight: 1.0,
            access_distance_m: 300.0,
            green_weight: 0.2,
            property_tax_rate: 0.002,
            road_upkeep_per_lane_km: 5_000_000.0,
            car_share_max: 0.7,
            car_share_min: 0.2,
            car_share_half_density: 15_000.0,
        }
    }
}

/// 거주 밀도(명/㎢) → 승용차 분담률.
pub fn car_share_for_density(density_per_km2: f64, p: &EconomyParams) -> f64 {
    p.car_share_min
        + (p.car_share_max - p.car_share_min)
            / (1.0 + density_per_km2.max(0.0) / p.car_share_half_density)
}

/// 셀(행 0 = 북쪽)마다 토지가치 (원/㎡). 구역 격자와 같은 크기.
pub fn land_values(world: &World, p: &EconomyParams) -> Vec<f64> {
    let z = &world.zones;
    let g = &world.road_graph;
    // 간선 선분들
    let segs: Vec<((f64, f64), (f64, f64))> = g
        .edges
        .iter()
        .filter(|e| !e.removed && is_major_road(&e.highway))
        .flat_map(|e| e.points.windows(2).map(|w| (w[0], w[1])))
        .collect();
    let seg_dist = |(a, b): ((f64, f64), (f64, f64)), x: f64, y: f64| {
        let (vx, vy) = (b.0 - a.0, b.1 - a.1);
        let len2 = vx * vx + vy * vy;
        let t = if len2 > 0.0 {
            (((x - a.0) * vx + (y - a.1) * vy) / len2).clamp(0.0, 1.0)
        } else {
            0.0
        };
        norm(a.0 + vx * t - x, a.1 + vy * t - y)
    };
    let mut out = Vec::with_capacity((z.rows * z.cols) as usize);
    for r in 0..z.rows {
        for c in 0..z.cols {
            let (x, y) = z.cell_center(r, c);
            let d = segs
                .iter()
                .map(|&s| seg_dist(s, x, y))
                .fold(f64::INFINITY, f64::min);
            let access = if d.is_finite() {
                1.0 / (1.0 + d / p.access_distance_m)
            } else {
                0.0
            };
            let green = if z.get(r, c) == Some(Zone::Park) {
                1.0
            } else {
                0.0
            };
            out.push(
                p.base_land_value
                    * (1.0 + p.access_weight * access)
                    * (1.0 + p.green_weight * green),
            );
        }
    }
    out
}

/// 계절 재정.
#[derive(Debug, Clone, PartialEq, Default)]
pub struct SeasonBudget {
    pub property_tax: f64,
    pub road_upkeep: f64,
    pub policy_cost: f64,
    pub balance: f64,
    pub debt_after: f64,
    pub mean_land_value: f64,
}

/// 한 계절 재정을 계산한다. `debt_before`는 이전까지 쌓인 부채(양수).
pub fn season_budget(
    world: &World,
    p: &EconomyParams,
    policy_cost: f64,
    debt_before: f64,
) -> SeasonBudget {
    let lv = land_values(world, p);
    let cell_area = world.zones.resolution_m * world.zones.resolution_m;
    let property_tax = lv.iter().map(|v| v * cell_area).sum::<f64>() * p.property_tax_rate / 4.0;
    let road_upkeep = world.road_graph.stats().lane_km * p.road_upkeep_per_lane_km;
    let balance = property_tax - road_upkeep - policy_cost;
    SeasonBudget {
        property_tax,
        road_upkeep,
        policy_cost,
        balance,
        debt_after: (debt_before - balance).max(0.0),
        mean_land_value: lv.iter().sum::<f64>() / lv.len().max(1) as f64,
    }
}
