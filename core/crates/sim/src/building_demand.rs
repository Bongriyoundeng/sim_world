//! 건물 → 통행 수요 (2단계 #36).
//!
//! - 주거 건물: 거주자 × 취업자 비율 × 승용차 비율 = 차로 출근하는 사람 (집 자리)
//! - 비주거 건물: 종사자 × 승용차 비율 = 차로 오는 종사자 (일자리 자리)
//! - 출근하는 사람은 남은 일자리 자리에 비례해 일터 건물을 고른다 (시드 고정)
//! - 방문: 비주거 건물의 하루 방문 통행(연면적 × 용도별 원단위)마다, 거주자 수에 비례해 출발 주거 건물을 고른다
//! - 시각: 출근 08:00·퇴근 18:00 (±30분, `DemandParams`), 방문 도착 14:00 ±3시간(08~21시), 머무는 시간 1시간 (가정)
//!
//! 근거와 가정: docs/design/research-log.md#land-use

use std::cmp::Reverse;
use std::collections::BTreeMap;

use dongne_world::land_use::{activity, BuildingActivity, BuildingUse, LandUseParams};
use dongne_world::road_graph::norm;

use crate::demand::{DemandParams, HOUR_MS};
use crate::traffic::Rng;
use crate::Simulation;

/// 방문 도착 시각 분포 (가정)
pub const VISIT_MEAN_MS: u64 = 14 * HOUR_MS;
pub const VISIT_SPREAD_MS: u64 = 3 * HOUR_MS;
pub const VISIT_EARLIEST_MS: u64 = 8 * HOUR_MS;
pub const VISIT_LATEST_MS: u64 = 21 * HOUR_MS;
pub const VISIT_DWELL_MS: u64 = HOUR_MS;

/// 용도별 합계.
#[derive(Debug, Clone, PartialEq, Default)]
pub struct UseTotals {
    pub buildings: u32,
    pub floor_area_m2: f64,
    pub residents: f64,
    pub workers: f64,
    /// 이 용도 건물에서 출발한 차량 통행
    pub trips_from: u32,
    /// 이 용도 건물로 도착한 차량 통행
    pub trips_to: u32,
}

#[derive(Debug, Clone, PartialEq, Default)]
pub struct BuildingDemandReport {
    pub buildings_used: u32,
    pub buildings_without_road: u32,
    pub residents: f64,
    pub workers: f64,
    pub car_commuters: u32,
    pub visit_trips: u32,
    /// 예약한 차량 통행 (출근·퇴근·방문·귀가)
    pub trips: u32,
    pub same_node: u32,
    pub unroutable: u32,
    pub by_use: BTreeMap<BuildingUse, UseTotals>,
}

/// 실수 기댓값을 정수로 (소수 부분은 확률로 반올림, 결정론).
fn stochastic_round(x: f64, rng: &mut Rng) -> u64 {
    let f = x.max(0.0).floor();
    let frac = x - f;
    let u = (rng.next_u64() >> 11) as f64 / (1u64 << 53) as f64;
    f as u64 + u64::from(u < frac)
}

/// 정규분포 근사 (균등분포 12개 합 − 6).
fn normal(rng: &mut Rng) -> f64 {
    (0..12)
        .map(|_| (rng.next_u64() >> 11) as f64 / (1u64 << 53) as f64)
        .sum::<f64>()
        - 6.0
}

fn sample(rng: &mut Rng, mean: u64, spread: u64, lo: u64, hi: u64) -> u64 {
    (mean as f64 + normal(rng) * spread as f64).clamp(lo as f64, hi as f64) as u64
}

/// 남은 자리 `slots`에 비례해 하나를 고르고 그 자리를 하나 줄인다.
fn pick(slots: &mut [u64], total: &mut u64, rng: &mut Rng) -> Option<usize> {
    if *total == 0 {
        return None;
    }
    let mut r = rng.below(*total);
    for (i, s) in slots.iter_mut().enumerate() {
        if r < *s {
            *s -= 1;
            *total -= 1;
            return Some(i);
        }
        r -= *s;
    }
    None
}

struct Site {
    act: BuildingActivity,
    node: u32,
}

impl Simulation {
    /// 건물 용도·연면적으로 `day_start_ms` 하루의 차량 통행(출퇴근·방문)을 예약한다.
    pub fn add_building_demand_at(
        &mut self,
        lp: &LandUseParams,
        dp: &DemandParams,
        seed: u64,
        day_start_ms: u64,
    ) -> BuildingDemandReport {
        let g = &self.world.road_graph;
        let comps = g.strongly_connected_components();
        let mut rep = BuildingDemandReport::default();
        let Some(reach) = comps.iter().max_by_key(|c| (c.len(), Reverse(c[0]))) else {
            return rep;
        };

        // 건물 → 활동량, 가까운 도로 노드
        let mut sites: Vec<Site> = Vec::new();
        for b in &self.world.buildings {
            let act = activity(b, lp);
            let u = act.use_.expect("분류됨");
            let t = rep.by_use.entry(u).or_default();
            t.buildings += 1;
            t.floor_area_m2 += act.floor_area_m2;
            t.residents += act.residents;
            t.workers += act.workers;
            rep.residents += act.residents;
            rep.workers += act.workers;
            if act.residents == 0.0 && act.workers == 0.0 && act.visit_trips == 0.0 {
                continue;
            }
            let (x, y) = act.centroid;
            let node = reach
                .iter()
                .map(|&id| {
                    let n = &g.nodes[id as usize];
                    (norm(n.x - x, n.y - y), id)
                })
                .filter(|&(d, _)| d <= dp.max_access_m)
                .min_by(|a, b| a.0.total_cmp(&b.0).then(a.1.cmp(&b.1)));
            match node {
                Some((_, node)) => sites.push(Site { act, node }),
                None => rep.buildings_without_road += 1,
            }
        }
        rep.buildings_used = sites.len() as u32;

        let mut rng = Rng::new(seed);
        // 집 자리·일자리 자리 (승용차)
        let homes: Vec<u64> = sites
            .iter()
            .map(|s| stochastic_round(s.act.residents * lp.employed_share * lp.car_share, &mut rng))
            .collect();
        let mut jobs: Vec<u64> = sites
            .iter()
            .map(|s| stochastic_round(s.act.workers * lp.car_share, &mut rng))
            .collect();
        let mut jobs_left: u64 = jobs.iter().sum();

        // (출발 건물, 도착 건물, 출발 시각)
        let mut trips: Vec<(usize, usize, u64)> = Vec::new();
        'homes: for (h, &n) in homes.iter().enumerate() {
            for _ in 0..n {
                let Some(w) = pick(&mut jobs, &mut jobs_left, &mut rng) else {
                    break 'homes;
                };
                rep.car_commuters += 1;
                let leave = sample(
                    &mut rng,
                    dp.morning_mean_ms,
                    dp.spread_ms,
                    0,
                    crate::demand::DAY_MS - 1,
                );
                let back = sample(
                    &mut rng,
                    dp.evening_mean_ms,
                    dp.spread_ms,
                    leave + 1,
                    crate::demand::DAY_MS - 1,
                );
                trips.push((h, w, leave));
                trips.push((w, h, back));
            }
        }

        // 방문: 거주자 수에 비례해 출발 집을 고른다 (거주자 10명 = 1 가중치 단위)
        let res_weight: Vec<u64> = sites
            .iter()
            .map(|s| (s.act.residents * 10.0).round() as u64)
            .collect();
        let res_total: u64 = res_weight.iter().sum();
        for (d, s) in sites.iter().enumerate() {
            let n = stochastic_round(s.act.visit_trips, &mut rng);
            for _ in 0..n {
                if res_total == 0 {
                    break;
                }
                let mut r = rng.below(res_total);
                let o = res_weight
                    .iter()
                    .position(|&w| {
                        if r < w {
                            true
                        } else {
                            r -= w;
                            false
                        }
                    })
                    .expect("가중치 안");
                let arrive = sample(
                    &mut rng,
                    VISIT_MEAN_MS,
                    VISIT_SPREAD_MS,
                    VISIT_EARLIEST_MS,
                    VISIT_LATEST_MS,
                );
                rep.visit_trips += 1;
                trips.push((o, d, arrive));
                trips.push((d, o, arrive + VISIT_DWELL_MS));
            }
        }

        for (o, d, t) in trips {
            let (so, sd) = (&sites[o], &sites[d]);
            if so.node == sd.node {
                rep.same_node += 1;
                continue;
            }
            if self
                .traffic
                .add_trip(so.node, sd.node, day_start_ms + t)
                .is_some()
            {
                rep.trips += 1;
                rep.by_use
                    .entry(so.act.use_.expect("분류됨"))
                    .or_default()
                    .trips_from += 1;
                rep.by_use
                    .entry(sd.act.use_.expect("분류됨"))
                    .or_default()
                    .trips_to += 1;
            } else {
                rep.unroutable += 1;
            }
        }
        rep
    }
}
