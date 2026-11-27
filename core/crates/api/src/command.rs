//! 클라이언트·AI → 코어 명령. JSON 예시: `schema/commands/examples/`.
//!
//! JSON 형식은 `{"type": "add_road", ...}`처럼 `type` 필드로 종류를 구분한다.

use serde::{Deserialize, Serialize};

/// 로컬 미터 좌표 (x 동쪽, y 북쪽).
#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
pub struct Point {
    pub x: f64,
    pub y: f64,
}

/// 새 도로의 끝. 기존 노드 번호나 좌표로 준다.
/// 좌표는 가까운 노드(코어의 스냅 반경 안)에 붙고, 없으면 새 노드가 된다.
#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum RoadEnd {
    Node(u32),
    Point(Point),
}

/// 구역 종류.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Zone {
    Residential,
    Commercial,
    Office,
    Park,
}

/// 구역을 칠할 범위. 셀은 `[행, 열]` (terrain과 같은 격자, 행 0 = 북쪽).
/// 폴리곤은 셀 중심이 안에 들어가는 셀을 칠한다.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ZoneArea {
    Cells(Vec<[u32; 2]>),
    Polygon(Vec<Point>),
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum Command {
    /// 두 끝을 잇는 직선 도로를 놓는다.
    AddRoad {
        from: RoadEnd,
        to: RoadEnd,
        highway: String,
        /// 양방향 합계 차선 수. 없으면 유형별 기본값.
        #[serde(default)]
        lanes: Option<u32>,
        #[serde(default)]
        oneway: bool,
    },
    /// 도로 엣지를 없앤다. 그 위에 있던 차량은 끝까지 빠져나간다.
    RemoveRoad { edge: u32 },
    /// 범위의 셀에 구역을 지정한다. `zone`이 null이면 지정을 지운다.
    SetZone { area: ZoneArea, zone: Option<Zone> },
}

/// 명령 처리 결과. 코어 → 클라이언트·AI 이벤트.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct CommandOutcome {
    /// `submit`이 돌려준 번호
    pub id: u64,
    /// 적용된 틱
    pub tick: u64,
    pub result: CommandResult,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(tag = "status", rename_all = "snake_case")]
pub enum CommandResult {
    /// 적용됨. 새로 생긴 엣지·노드, 바뀐 셀 수, 경로를 다시 찾은 차량 수
    Applied {
        #[serde(default, skip_serializing_if = "Option::is_none")]
        edge: Option<u32>,
        #[serde(default, skip_serializing_if = "Vec::is_empty")]
        new_nodes: Vec<u32>,
        #[serde(default, skip_serializing_if = "is_zero")]
        cells: u32,
        #[serde(default, skip_serializing_if = "is_zero")]
        rerouted: u32,
    },
    /// 거부됨. 상태는 바뀌지 않는다.
    Rejected { reason: String },
}

fn is_zero(n: &u32) -> bool {
    *n == 0
}

#[cfg(test)]
mod tests {
    use super::*;

    fn round_trip(json: &str) -> Command {
        let c: Command = serde_json::from_str(json).unwrap();
        let back: Command = serde_json::from_str(&serde_json::to_string(&c).unwrap()).unwrap();
        assert_eq!(c, back);
        c
    }

    #[test]
    fn schema_examples_parse() {
        let add = round_trip(include_str!(
            "../../../../schema/commands/examples/add_road.json"
        ));
        assert!(matches!(add, Command::AddRoad { oneway: false, .. }));
        let remove = round_trip(include_str!(
            "../../../../schema/commands/examples/remove_road.json"
        ));
        assert_eq!(remove, Command::RemoveRoad { edge: 42 });
        let cells = round_trip(include_str!(
            "../../../../schema/commands/examples/set_zone_cells.json"
        ));
        assert!(matches!(
            cells,
            Command::SetZone {
                zone: Some(Zone::Residential),
                ..
            }
        ));
        let poly = round_trip(include_str!(
            "../../../../schema/commands/examples/set_zone_polygon.json"
        ));
        assert!(matches!(
            poly,
            Command::SetZone {
                area: ZoneArea::Polygon(_),
                zone: Some(Zone::Commercial)
            }
        ));
    }

    #[test]
    fn outcome_json_shape() {
        let o = CommandOutcome {
            id: 3,
            tick: 10,
            result: CommandResult::Rejected {
                reason: "없는 엣지".into(),
            },
        };
        let v: serde_json::Value = serde_json::to_value(&o).unwrap();
        assert_eq!(v["result"]["status"], "rejected");
        let a = CommandResult::Applied {
            edge: Some(5),
            new_nodes: vec![],
            cells: 0,
            rerouted: 2,
        };
        let v = serde_json::to_value(&a).unwrap();
        assert_eq!(
            v,
            serde_json::json!({"status": "applied", "edge": 5, "rerouted": 2})
        );
    }
}
