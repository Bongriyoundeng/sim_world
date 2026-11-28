//! 맵 패키지를 읽어 월드를 만든다.

use std::fmt;
use std::path::Path;

pub use dongne_api::Manifest;

pub mod buildings;
pub mod land_use;
pub mod road_graph;
pub mod zones;
pub use buildings::Building;
pub use road_graph::{GraphError, GraphStats, RoadGraph};
pub use zones::{Zone, ZoneMap};

/// 맵 패키지 로드 오류.
#[derive(Debug)]
pub enum LoadError {
    Io(std::io::Error),
    Parse(serde_json::Error),
    UnsupportedVersion(u32),
    MissingAttribution,
    Roads(GraphError),
}

impl fmt::Display for LoadError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            LoadError::Io(e) => write!(f, "맵 패키지를 읽을 수 없음: {e}"),
            LoadError::Parse(e) => write!(f, "manifest.json 형식 오류: {e}"),
            LoadError::UnsupportedVersion(v) => write!(f, "지원하지 않는 format_version: {v}"),
            LoadError::MissingAttribution => write!(f, "manifest.json에 attribution(출처)이 없음"),
            LoadError::Roads(e) => write!(f, "{e}"),
        }
    }
}

impl std::error::Error for LoadError {}

/// 지원하는 맵 패키지 형식 버전.
pub const FORMAT_VERSION: u32 = 0;

/// 맵 패키지에서 읽은 월드.
#[derive(Debug, Clone)]
pub struct World {
    pub manifest: Manifest,
    /// `roads.geojson`에서 만든 차량 도로 그래프. 파일이 없으면 비어 있다.
    pub road_graph: RoadGraph,
    /// 셀 단위 구역 지정 (terrain과 같은 격자). 처음에는 비어 있다.
    pub zones: ZoneMap,
    /// `buildings.geojson`의 건물. 파일이 없으면 비어 있다.
    pub buildings: Vec<Building>,
}

/// `manifest.json` 문자열을 검증해 읽는다.
pub fn parse_manifest(json: &str) -> Result<Manifest, LoadError> {
    let m: Manifest = serde_json::from_str(json).map_err(LoadError::Parse)?;
    if m.format_version != FORMAT_VERSION {
        return Err(LoadError::UnsupportedVersion(m.format_version));
    }
    if m.attribution.iter().all(|a| a.trim().is_empty()) {
        return Err(LoadError::MissingAttribution);
    }
    Ok(m)
}

/// 맵 패키지 폴더를 읽는다.
pub fn load_map(dir: &Path) -> Result<World, LoadError> {
    let json = std::fs::read_to_string(dir.join("manifest.json")).map_err(LoadError::Io)?;
    let manifest = parse_manifest(&json)?;
    let roads_path = dir.join("roads.geojson");
    let road_graph = if roads_path.exists() {
        let roads = std::fs::read_to_string(roads_path).map_err(LoadError::Io)?;
        RoadGraph::from_geojson(&roads, manifest.extent_m.width, manifest.extent_m.height)
            .map_err(LoadError::Roads)?
    } else {
        RoadGraph::default()
    };
    let zones = ZoneMap::new(
        manifest.extent_m.width,
        manifest.extent_m.height,
        manifest.terrain.resolution_m,
    );
    let buildings_path = dir.join("buildings.geojson");
    let buildings = if buildings_path.exists() {
        let json = std::fs::read_to_string(buildings_path).map_err(LoadError::Io)?;
        buildings::parse_buildings(&json).map_err(LoadError::Roads)?
    } else {
        Vec::new()
    };
    Ok(World {
        manifest,
        road_graph,
        zones,
        buildings,
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    fn manifest(attribution: &str) -> String {
        format!(
            r#"{{"format_version":0,"name":"t","origin":{{"lat":0,"lon":0}},"crs":"local-meters",
            "extent_m":{{"width":1,"height":1}},
            "terrain":{{"file":"terrain.png","resolution_m":30,"min_elevation_m":0,"max_elevation_m":1}},
            "attribution":[{attribution}]}}"#
        )
    }

    #[test]
    fn accepts_manifest_with_attribution() {
        assert!(parse_manifest(&manifest(r#""© OpenStreetMap contributors""#)).is_ok());
    }

    #[test]
    fn rejects_manifest_without_attribution() {
        assert!(matches!(
            parse_manifest(&manifest("")),
            Err(LoadError::MissingAttribution)
        ));
    }
}
