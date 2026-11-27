//! 코어와 클라이언트·AI가 주고받는 타입. `schema/`와 일치해야 한다.

use serde::{Deserialize, Serialize};

pub mod command;
pub use command::{Command, CommandOutcome, CommandResult, Point, RoadEnd, Zone, ZoneArea};

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

/// 토지 피복 래스터의 채널 의미.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct LandcoverChannels {
    pub r: String,
    pub g: String,
    pub b: String,
}

/// 토지 피복 레이어 정보 (8비트 RGB, terrain과 같은 격자). 값 / `scale` = 셀 면적 비율 0~1.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct LandcoverInfo {
    pub file: String,
    pub resolution_m: f64,
    pub channels: LandcoverChannels,
    pub scale: f64,
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
    /// 선택 항목. 없는 맵 패키지도 그대로 읽힌다.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub landcover: Option<LandcoverInfo>,
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
        assert!(m.landcover.is_none());
    }

    #[test]
    fn manifest_with_landcover() {
        let json = r#"{
            "format_version": 0,
            "name": "sample",
            "origin": {"lat": 37.5, "lon": 127.0},
            "crs": "local-meters",
            "extent_m": {"width": 1500, "height": 1500},
            "terrain": {"file": "terrain.png", "resolution_m": 30, "min_elevation_m": 0, "max_elevation_m": 120},
            "landcover": {"file": "landcover.png", "resolution_m": 30,
                          "channels": {"r": "impervious", "g": "canopy", "b": "water"}, "scale": 255},
            "attribution": ["© OpenStreetMap contributors (ODbL 1.0)"]
        }"#;
        let m: Manifest = serde_json::from_str(json).unwrap();
        let lc = m.landcover.as_ref().unwrap();
        assert_eq!(lc.file, "landcover.png");
        assert_eq!(lc.channels.g, "canopy");
        let back: Manifest = serde_json::from_str(&serde_json::to_string(&m).unwrap()).unwrap();
        assert_eq!(m, back);
    }
}
