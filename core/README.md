# core/ — 시뮬레이션 코어 (Rust)

| crate | 역할 |
| --- | --- |
| `crates/api` | 명령·스냅샷·맵 패키지 타입 (`schema/`와 일치) |
| `crates/world` | 맵 패키지 읽기, 도로 그래프, 지형 |
| `crates/sim` | 틱 루프, 교통·경제·인구 |
| `crates/py` | Python 바인딩 (PyO3) — 0단계 6번 작업에서 추가 |
| `crates/godot` | Godot 바인딩 (GDExtension) — 0단계 7번 작업에서 추가 |

```sh
cd core && cargo test
```

## 도로 그래프 (`crates/world/src/road_graph.rs`)

`load_map`이 `roads.geojson`을 읽어 `World::road_graph`를 만듭니다. [ADR 0004](../docs/adr/0004-traffic-model.md)의 차선 대기열 모델 입력입니다.

- **포함**: motorway·trunk·primary·secondary·tertiary와 각 `*_link`, unclassified, residential, living_street, service. 나머지(footway, path, steps, cycleway, pedestrian 등)는 빼고 유형별 개수만 `excluded`에 남깁니다.
- **노드**: 선의 끝점과, 두 번 이상 쓰인 정점(0.5 m 격자로 스냅해 비교). 구역 경계에 붙은 끝점은 `boundary`.
- **엣지**: 노드 사이 구간. 길이(m), 방향별 차선 수(`lanes_forward`, `lanes_backward`), highway 유형, 이름, 자유 흐름 속도, 선 좌표.
- **차선**: 엣지·방향·순번마다 하나. 길이와 대기열 용량(길이 ÷ 7.5 m, 최소 1대).
- **차선 수**: `oneway`면 모두 정방향, 아니면 반씩(홀수면 정방향 +1, 방향별 최소 1). `lanes`가 없으면 유형별 기본값.
- **결정론**: 노드 ID는 스냅 좌표 순, 엣지 ID는 (osm_id, 첫 좌표) 순으로 처리한 순서. 피처 순서가 바뀌어도 같은 그래프가 나옵니다 (테스트로 확인).
- 차선 수 기본값, 차량 간격, 자유 흐름 속도는 근거가 없는 **가정**입니다 ([research-log.md](../docs/design/research-log.md#road-lanes)).

`maps/pyeongtaek-station` (OSM 2026-11-27): 노드 908, 엣지 1,272, 차선 2,588, 경계 노드 87, 총 차선 137.5 km, 가장 큰 강연결 요소 94.3 %. 통합 테스트 `crates/world/tests/pyeongtaek_station.rs`는 하한 0.8로 확인합니다.
