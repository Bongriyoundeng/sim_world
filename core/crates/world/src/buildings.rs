//! `buildings.geojson` 읽기: 건물 바닥 다각형과 표시 높이.
//!
//! OSM에 층수·높이가 거의 없어(평택역 맵 3048개 중 층수 3개, 높이 0개) 표시 높이는
//! 기본값을 쓴다. 가정: docs/design/research-log.md#building-height

use serde_json::Value;

use crate::road_graph::GraphError;

/// 한 층 높이 (m, 가정).
pub const STOREY_M: f64 = 3.0;

#[derive(Debug, Clone, PartialEq)]
pub struct Building {
    pub osm_id: i64,
    /// OSM `building` 값 (yes, apartments, school, …)
    pub kind: String,
    pub levels: Option<f64>,
    /// 표시 높이 (m): `height` 태그 → `building:levels` × 3 m → 유형별 기본값 순
    pub height_m: f64,
    /// 바깥 고리 (로컬 미터, 마지막 점 = 첫 점 아님)
    pub footprint: Vec<(f64, f64)>,
}

/// 층수·높이 태그가 없을 때 유형별 기본 높이 (m, 가정).
pub fn default_height_m(kind: &str) -> f64 {
    match kind {
        "apartments" => 15.0 * STOREY_M, // 아파트 단지 15층
        "house" | "detached" | "residential" | "semidetached_house" => 2.0 * STOREY_M,
        "commercial" | "retail" | "office" => 4.0 * STOREY_M,
        "school" | "university" | "public" | "civic" | "hospital" => 4.0 * STOREY_M,
        "train_station" | "industrial" | "warehouse" => 2.0 * STOREY_M, // 층고 높은 1~2층
        "roof" | "shed" | "garage" | "kiosk" => 1.0 * STOREY_M,
        "construction" => 1.0 * STOREY_M,
        _ => 3.0 * STOREY_M, // building=yes 등: 원도심 저층 상가주택 3층
    }
}

pub fn parse_buildings(json: &str) -> Result<Vec<Building>, GraphError> {
    let v: Value = serde_json::from_str(json).map_err(GraphError::Parse)?;
    let features = v["features"]
        .as_array()
        .ok_or_else(|| GraphError::Format("buildings.geojson에 features 배열이 없음".into()))?;
    let mut out = Vec::with_capacity(features.len());
    for f in features {
        let p = &f["properties"];
        let Some(ring) = f["geometry"]["coordinates"][0].as_array() else {
            continue;
        };
        let mut footprint: Vec<(f64, f64)> = ring
            .iter()
            .filter_map(|c| Some((c[0].as_f64()?, c[1].as_f64()?)))
            .collect();
        if footprint.len() >= 2 && footprint.first() == footprint.last() {
            footprint.pop();
        }
        if footprint.len() < 3 {
            continue;
        }
        let kind = p["building"].as_str().unwrap_or("yes").to_string();
        let levels = p["building:levels"].as_f64().filter(|&l| l > 0.0);
        let height_m = p["height"]
            .as_f64()
            .filter(|&h| h > 0.0)
            .or(levels.map(|l| l * STOREY_M))
            .unwrap_or_else(|| default_height_m(&kind));
        out.push(Building {
            osm_id: p["osm_id"].as_i64().unwrap_or(0),
            kind,
            levels,
            height_m,
            footprint,
        });
    }
    Ok(out)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn heights_from_tags_then_defaults() {
        let json = r#"{"type":"FeatureCollection","features":[
          {"type":"Feature","properties":{"osm_id":1,"building":"apartments","building:levels":null,"height":null},
           "geometry":{"type":"Polygon","coordinates":[[[0,0],[10,0],[10,10],[0,10],[0,0]]]}},
          {"type":"Feature","properties":{"osm_id":2,"building":"yes","building:levels":5,"height":null},
           "geometry":{"type":"Polygon","coordinates":[[[0,0],[10,0],[10,10],[0,0]]]}},
          {"type":"Feature","properties":{"osm_id":3,"building":"yes","building:levels":5,"height":12.5},
           "geometry":{"type":"Polygon","coordinates":[[[0,0],[10,0],[10,10],[0,0]]]}}
        ]}"#;
        let b = parse_buildings(json).unwrap();
        assert_eq!(b.len(), 3);
        assert_eq!(b[0].height_m, 45.0);
        assert_eq!(b[0].footprint.len(), 4); // 닫는 점 제거
        assert_eq!(b[1].height_m, 15.0);
        assert_eq!(b[2].height_m, 12.5);
    }
}
