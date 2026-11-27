//! 코어와 클라이언트·AI가 주고받는 타입. `schema/`와 일치해야 한다.

use serde::{Deserialize, Serialize};

/// 지리 기준점 (위경도). 로컬 좌표의 원점이다.
#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
pub struct Origin {
    pub lat: f64,
    pub lon: f64,
}

/// 맵 패키지 범위 (미터).
#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
pub struct Extent {
    pub width: f64,
    pub height: f64,
}

/// 지형 높이맵 정보.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct TerrainInfo {
    pub file: String,
    pub resolution_m: f64,
    pub min_elevation_m: f64,
    pub max_elevation_m: f64,
}

/// 맵 패키지 `manifest.json` (v0).
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Manifest {
    pub format_version: u32,
    pub name: String,
    pub origin: Origin,
    pub crs: String,
    pub extent_m: Extent,
    pub terrain: TerrainInfo,
    pub attribution: Vec<String>,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn manifest_round_trip() {
        let json = r#"{
            "format_version": 0,
            "name": "sample",
            "origin": {"lat": 37.5, "lon": 127.0},
            "crs": "local-meters",
            "extent_m": {"width": 1500, "height": 1500},
            "terrain": {"file": "terrain.png", "resolution_m": 30, "min_elevation_m": 0, "max_elevation_m": 120},
            "attribution": ["© OpenStreetMap contributors (ODbL 1.0)"]
        }"#;
        let m: Manifest = serde_json::from_str(json).unwrap();
        assert_eq!(m.name, "sample");
        let back: Manifest = serde_json::from_str(&serde_json::to_string(&m).unwrap()).unwrap();
        assert_eq!(m, back);
    }
}
