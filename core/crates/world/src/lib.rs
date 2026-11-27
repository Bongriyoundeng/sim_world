//! 맵 패키지를 읽어 월드를 만든다.

use std::fmt;
use std::path::Path;

pub use dongne_api::Manifest;

/// 맵 패키지 로드 오류.
#[derive(Debug)]
pub enum LoadError {
    Io(std::io::Error),
    Parse(serde_json::Error),
    UnsupportedVersion(u32),
    MissingAttribution,
}

impl fmt::Display for LoadError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            LoadError::Io(e) => write!(f, "맵 패키지를 읽을 수 없음: {e}"),
            LoadError::Parse(e) => write!(f, "manifest.json 형식 오류: {e}"),
            LoadError::UnsupportedVersion(v) => write!(f, "지원하지 않는 format_version: {v}"),
            LoadError::MissingAttribution => write!(f, "manifest.json에 attribution(출처)이 없음"),
        }
    }
}

impl std::error::Error for LoadError {}

/// 지원하는 맵 패키지 형식 버전.
pub const FORMAT_VERSION: u32 = 0;

/// 맵 패키지에서 읽은 월드. 0단계에서는 manifest만 담는다.
#[derive(Debug, Clone)]
pub struct World {
    pub manifest: Manifest,
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
    Ok(World {
        manifest: parse_manifest(&json)?,
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
