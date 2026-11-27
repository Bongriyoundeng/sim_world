//! 구역 → 통근 수요 (주거 → 업무·상업). 1단계는 출근·퇴근 두 통행만 만든다.
//!
//! 1. 주거 셀마다 통근자 `residents_per_cell`명, 업무·상업 셀마다 일자리를 둔다.
//! 2. 통근자는 셀 순서대로, 남은 일자리 수에 비례한 확률로 일터 셀을 고른다 (시드 고정).
//!    일자리가 모자라면 남는 주민은 통근하지 않는다.
//! 3. 집·일터 셀은 셀 중심에서 가장 가까운 도로 노드(가장 큰 강연결 요소 안, `max_access_m` 이내)에 붙는다.
//! 4. 출근 시각 ~ 정규분포(평균 `morning_mean_ms`, 표준편차 `spread_ms`), 퇴근도 같은 방식.
//!
//! 파라미터는 모두 가정이다: docs/design/research-log.md#commute-demand

use std::cmp::Reverse;

use dongne_world::{RoadGraph, Zone, ZoneMap};

use crate::traffic::Rng;

pub const HOUR_MS: u64 = 3_600_000;
pub const DAY_MS: u64 = 24 * HOUR_MS;

/// 통근 수요 파라미터. 시각은 자정(시뮬레이션 0 ms) 기준.
#[derive(Debug, Clone, PartialEq)]
pub struct DemandParams {
    pub residents_per_cell: u32,
    pub jobs_per_office_cell: u32,
    pub jobs_per_commercial_cell: u32,
    pub morning_mean_ms: u64,
    pub evening_mean_ms: u64,
    pub spread_ms: u64,
    pub max_access_m: f64,
}

impl Default for DemandParams {
    /// 가정값 (research-log.md#commute-demand)
    fn default() -> Self {
        Self {
            residents_per_cell: 10,
            jobs_per_office_cell: 30,
            jobs_per_commercial_cell: 10,
            morning_mean_ms: 8 * HOUR_MS,
            evening_mean_ms: 18 * HOUR_MS,
            spread_ms: HOUR_MS / 2,
            max_access_m: 300.0,
        }
    }
}

#[derive(Debug, Clone, PartialEq)]
pub struct Commuter {
    pub home_cell: (u32, u32),
    pub work_cell: (u32, u32),
    pub home_node: u32,
    pub work_node: u32,
    pub leave_home_ms: u64,
    pub leave_work_ms: u64,
}

#[derive(Debug, Clone, PartialEq, Default)]
pub struct DemandSummary {
    pub residents: u32,
    pub jobs: u32,
    pub commuters: u32,
    /// 가까운 도로 노드가 없어 빠진 주거·일자리 셀 수
    pub cells_without_road: u32,
}

fn jobs_in(zone: Zone, p: &DemandParams) -> u32 {
    match zone {
        Zone::Office => p.jobs_per_office_cell,
        Zone::Commercial => p.jobs_per_commercial_cell,
        _ => 0,
    }
}

/// 평균 0, 표준편차 1에 가까운 값 (Irwin–Hall: 균등분포 12개 합 − 6). 결정론적.
fn standard_normal(rng: &mut Rng) -> f64 {
    (0..12)
        .map(|_| (rng.next_u64() >> 11) as f64 / (1u64 << 53) as f64)
        .sum::<f64>()
        - 6.0
}

fn sample_time(rng: &mut Rng, mean_ms: u64, spread_ms: u64) -> u64 {
    let t = mean_ms as f64 + standard_normal(rng) * spread_ms as f64;
    t.clamp(0.0, (DAY_MS - 1) as f64) as u64
}

/// 셀 중심에서 가장 가까운 노드 (후보 노드 중, `max_m` 이내, 동률이면 번호가 작은 노드).
fn access_node(g: &RoadGraph, candidates: &[u32], x: f64, y: f64, max_m: f64) -> Option<u32> {
    candidates
        .iter()
        .map(|&id| {
            let n = &g.nodes[id as usize];
            ((n.x - x).hypot(n.y - y), id)
        })
        .filter(|&(d, _)| d <= max_m)
        .min_by(|a, b| a.0.total_cmp(&b.0).then(a.1.cmp(&b.1)))
        .map(|(_, id)| id)
}

/// 구역 맵에서 통근자를 만든다. 같은 입력·시드면 같은 결과.
pub fn generate_commuters(
    zones: &ZoneMap,
    g: &RoadGraph,
    p: &DemandParams,
    seed: u64,
) -> (Vec<Commuter>, DemandSummary) {
    let mut summary = DemandSummary::default();
    let comps = g.strongly_connected_components();
    let Some(reachable) = comps.iter().max_by_key(|c| (c.len(), Reverse(c[0]))) else {
        return (Vec::new(), summary);
    };

    let mut homes: Vec<((u32, u32), u32)> = Vec::new(); // (셀, 노드)
    let mut works: Vec<((u32, u32), u32, u32)> = Vec::new(); // (셀, 노드, 남은 일자리)
    for (r, c, zone) in zones.zoned() {
        let is_home = zone == Zone::Residential;
        let jobs = jobs_in(zone, p);
        if !is_home && jobs == 0 {
            continue;
        }
        let (x, y) = zones.cell_center(r, c);
        let Some(node) = access_node(g, reachable, x, y, p.max_access_m) else {
            summary.cells_without_road += 1;
            continue;
        };
        if is_home {
            homes.push(((r, c), node));
            summary.residents += p.residents_per_cell;
        } else {
            works.push(((r, c), node, jobs));
            summary.jobs += jobs;
        }
    }

    let mut rng = Rng::new(seed);
    let mut remaining: u64 = works.iter().map(|w| w.2 as u64).sum();
    let mut out = Vec::new();
    'homes: for &(home_cell, home_node) in &homes {
        for _ in 0..p.residents_per_cell {
            if remaining == 0 {
                break 'homes;
            }
            // 남은 일자리에 비례해 일터를 고른다
            let mut pick = rng.below(remaining);
            let w = works
                .iter_mut()
                .find(|w| {
                    if pick < w.2 as u64 {
                        true
                    } else {
                        pick -= w.2 as u64;
                        false
                    }
                })
                .expect("남은 일자리 안에서 고름");
            w.2 -= 1;
            remaining -= 1;
            let leave_home_ms = sample_time(&mut rng, p.morning_mean_ms, p.spread_ms);
            let leave_work_ms =
                sample_time(&mut rng, p.evening_mean_ms, p.spread_ms).max(leave_home_ms + 1);
            out.push(Commuter {
                home_cell,
                work_cell: w.0,
                home_node,
                work_node: w.1,
                leave_home_ms,
                leave_work_ms,
            });
        }
    }
    summary.commuters = out.len() as u32;
    (out, summary)
}

/// 시간대별(1시간) 도로 위 차량 수: 그 시간 안에 한 번이라도 주행 중이던 차량 수.
pub fn vehicles_on_road_by_hour(vehicles: &[crate::traffic::Vehicle]) -> [u32; 24] {
    let mut h = [0u32; 24];
    for v in vehicles {
        let start = v.depart_ms;
        let end = v.arrive_ms.unwrap_or(DAY_MS - 1).min(DAY_MS - 1);
        for (hour, count) in h.iter_mut().enumerate() {
            let (a, b) = (hour as u64 * HOUR_MS, (hour as u64 + 1) * HOUR_MS);
            if start < b && end >= a {
                *count += 1;
            }
        }
    }
    h
}
