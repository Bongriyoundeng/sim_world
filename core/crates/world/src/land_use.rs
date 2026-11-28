//! 건물 용도·연면적 → 거주 인구·종사자·방문 수요 (2단계 #36).
//!
//! 연면적 = 바닥 면적 × 층수. 층수는 OSM `building:levels`, 없으면 표시 높이 ÷ 3 m(유형별 가정).
//! 주거 연면적은 거주 인구로, 비주거 연면적은 용도별 종사자와 방문 수요로 바꾼다.
//! 근거와 가정: docs/design/research-log.md#land-use, docs/design/balance.md

use crate::buildings::{Building, STOREY_M};

/// 건물 용도 (교통 발생 관점).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub enum BuildingUse {
    Residential,
    Commercial,
    Office,
    Public,
    Industrial,
    /// `building=yes` 등 용도 불명: 원도심 상가주택으로 보고 1층 상업 + 나머지 주거 (가정)
    Mixed,
    /// 차고·창고 지붕·공사 중 등 사람이 머물지 않음
    Inactive,
}

/// OSM `building` 값 → 용도.
pub fn classify(kind: &str) -> BuildingUse {
    match kind {
        "apartments" | "house" | "detached" | "residential" | "semidetached_house" | "terrace"
        | "dormitory" => BuildingUse::Residential,
        "commercial" | "retail" | "supermarket" | "kiosk" | "hotel" => BuildingUse::Commercial,
        "office" => BuildingUse::Office,
        "school" | "university" | "college" | "public" | "civic" | "hospital" | "government"
        | "train_station" | "church" | "kindergarten" => BuildingUse::Public,
        "industrial" | "warehouse" | "factory" => BuildingUse::Industrial,
        "roof" | "shed" | "garage" | "garages" | "construction" | "carport" | "hut" => {
            BuildingUse::Inactive
        }
        _ => BuildingUse::Mixed,
    }
}

/// 용도별 밀도와 통행 원단위. 모든 값은 docs/design/research-log.md#land-use 참고.
#[derive(Debug, Clone, PartialEq)]
pub struct LandUseParams {
    /// 주거 연면적 ㎡당 거주자 수의 역수: 1인당 주거면적 (㎡, 가정)
    pub residential_m2_per_person: f64,
    /// 종사자 1인당 연면적 (㎡): 사무 43.9 — 미국 EIA CBECS 2012 표 PBA1 사무 473 ft²
    pub office_m2_per_worker: f64,
    /// 상업 115.5 — 같은 표 상업(mercantile) 1,243 ft²
    pub commercial_m2_per_worker: f64,
    /// 공공·교육 (가정)
    pub public_m2_per_worker: f64,
    /// 산업·창고 (가정)
    pub industrial_m2_per_worker: f64,
    /// 거주자 중 통근하는 취업자 비율 (가정)
    pub employed_share: f64,
    /// 통근·방문 중 승용차로 가는 비율 (가정, #37에서 밀도에 따라 바꿈)
    pub car_share: f64,
    /// 연면적 1,000㎡당 하루 방문(차량) 통행 — 교통영향평가지침의 "용도별 24시간 단위면적당 원단위" 방식, 값은 가정
    pub visit_trips_per_1000m2: [f64; 3], // [상업, 공공, 사무]
}

impl Default for LandUseParams {
    fn default() -> Self {
        Self {
            residential_m2_per_person: 30.0,
            office_m2_per_worker: 473.0 * 0.092_903,
            commercial_m2_per_worker: 1_243.0 * 0.092_903,
            public_m2_per_worker: 100.0,
            industrial_m2_per_worker: 100.0,
            employed_share: 0.5,
            car_share: 0.5,
            visit_trips_per_1000m2: [20.0, 5.0, 2.0],
        }
    }
}

/// 건물 하나의 활동량.
#[derive(Debug, Clone, PartialEq, Default)]
pub struct BuildingActivity {
    pub use_: Option<BuildingUse>,
    pub floors: u32,
    pub footprint_m2: f64,
    pub floor_area_m2: f64,
    pub residential_m2: f64,
    pub nonresidential_m2: f64,
    pub residents: f64,
    pub workers: f64,
    /// 하루 방문 차량 통행 (도착 기준)
    pub visit_trips: f64,
    /// 바닥 다각형 무게중심 (로컬 미터)
    pub centroid: (f64, f64),
}

/// 다각형 넓이와 무게중심 (신발끈 공식, 사칙연산만 — 결정론).
pub fn area_centroid(poly: &[(f64, f64)]) -> (f64, (f64, f64)) {
    let n = poly.len();
    let (mut a2, mut cx, mut cy) = (0.0, 0.0, 0.0);
    for i in 0..n {
        let (x0, y0) = poly[i];
        let (x1, y1) = poly[(i + 1) % n];
        let cross = x0 * y1 - x1 * y0;
        a2 += cross;
        cx += (x0 + x1) * cross;
        cy += (y0 + y1) * cross;
    }
    if a2.abs() < 1e-9 {
        let sx: f64 = poly.iter().map(|p| p.0).sum();
        let sy: f64 = poly.iter().map(|p| p.1).sum();
        return (0.0, (sx / n as f64, sy / n as f64));
    }
    (a2.abs() / 2.0, (cx / (3.0 * a2), cy / (3.0 * a2)))
}

/// 건물의 층수: OSM 층수 태그, 없으면 표시 높이 ÷ 3 m (최소 1층).
pub fn floors(b: &Building) -> u32 {
    let f = b.levels.unwrap_or(b.height_m / STOREY_M);
    f.round().max(1.0) as u32
}

pub fn activity(b: &Building, p: &LandUseParams) -> BuildingActivity {
    let use_ = classify(&b.kind);
    let (footprint, centroid) = area_centroid(&b.footprint);
    let floors = floors(b);
    let total = footprint * floors as f64;
    let (res, nonres) = match use_ {
        BuildingUse::Residential => (total, 0.0),
        BuildingUse::Mixed => {
            let ground = footprint.min(total);
            (total - ground, ground)
        }
        BuildingUse::Inactive => (0.0, 0.0),
        _ => (0.0, total),
    };
    let per_worker = match use_ {
        BuildingUse::Office => p.office_m2_per_worker,
        BuildingUse::Commercial | BuildingUse::Mixed => p.commercial_m2_per_worker,
        BuildingUse::Public => p.public_m2_per_worker,
        BuildingUse::Industrial => p.industrial_m2_per_worker,
        _ => f64::INFINITY,
    };
    let visit_rate = match use_ {
        BuildingUse::Commercial | BuildingUse::Mixed => p.visit_trips_per_1000m2[0],
        BuildingUse::Public => p.visit_trips_per_1000m2[1],
        BuildingUse::Office => p.visit_trips_per_1000m2[2],
        _ => 0.0,
    };
    BuildingActivity {
        use_: Some(use_),
        floors,
        footprint_m2: footprint,
        floor_area_m2: total,
        residential_m2: res,
        nonresidential_m2: nonres,
        residents: res / p.residential_m2_per_person,
        workers: if per_worker.is_finite() {
            nonres / per_worker
        } else {
            0.0
        },
        visit_trips: nonres / 1000.0 * visit_rate,
        centroid,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn b(kind: &str, levels: Option<f64>, side: f64) -> Building {
        Building {
            osm_id: 1,
            kind: kind.into(),
            levels,
            height_m: crate::buildings::default_height_m(kind),
            footprint: vec![(0.0, 0.0), (side, 0.0), (side, side), (0.0, side)],
        }
    }

    #[test]
    fn area_and_centroid() {
        let (a, c) = area_centroid(&[(0.0, 0.0), (10.0, 0.0), (10.0, 20.0), (0.0, 20.0)]);
        assert_eq!(a, 200.0);
        assert_eq!(c, (5.0, 10.0));
    }

    #[test]
    fn same_floor_area_different_activity_by_use() {
        let p = LandUseParams::default();
        // 20 m × 20 m, 5층 → 연면적 2,000㎡
        let res = activity(&b("apartments", Some(5.0), 20.0), &p);
        let off = activity(&b("office", Some(5.0), 20.0), &p);
        let shop = activity(&b("retail", Some(5.0), 20.0), &p);
        for a in [&res, &off, &shop] {
            assert_eq!(a.floor_area_m2, 2_000.0);
        }
        assert!((res.residents - 2_000.0 / 30.0).abs() < 1e-9 && res.workers == 0.0);
        assert!(
            off.workers > shop.workers,
            "사무가 상업보다 종사자 밀도 높음"
        );
        assert!(shop.visit_trips > off.visit_trips && off.residents == 0.0);
        // 용도 불명(상가주택): 1층 상업 + 나머지 주거
        let mixed = activity(&b("yes", Some(3.0), 20.0), &p);
        assert_eq!(
            (mixed.nonresidential_m2, mixed.residential_m2),
            (400.0, 800.0)
        );
        // 층수 태그가 없으면 표시 높이 ÷ 3 m
        assert_eq!(floors(&b("apartments", None, 20.0)), 15);
        assert_eq!(activity(&b("garage", None, 20.0), &p).floor_area_m2, 400.0);
        assert_eq!(activity(&b("garage", None, 20.0), &p).residents, 0.0);
    }
}
