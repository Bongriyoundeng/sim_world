# core/ — 시뮬레이션 코어 (Rust)

| crate | 역할 |
| --- | --- |
| `crates/api` | 명령·스냅샷·맵 패키지 타입 (`schema/`와 일치) |
| `crates/world` | 맵 패키지 읽기, 도로 그래프, 지형 |
| `crates/sim` | 틱 루프, 교통·경제·인구 |
| `crates/py` | Python 바인딩 (PyO3, `import dongne_core`) — [README](crates/py/README.md) |
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

## 차량 이동 (`crates/sim/src/traffic.rs`)

[ADR 0004](../docs/adr/0004-traffic-model.md)의 이벤트 기반 차선 대기열 모델입니다. `Simulation::step(n)`이 n틱(1틱 = 100 ms)까지의 이벤트를 처리합니다.

- 차량 상태: 출발 대기 → 차선 주행(`Crossing`) → 차선 끝 대기열(`Queued`) → 맨 앞에서 교차로 통과 대기(`WaitingToAdvance`) → 다음 차선 … → 도착.
- 차선마다 대기열 용량(도로 그래프의 길이 ÷ 7.5 m)을 넘으면 들어가지 못하고 교차로에서 기다립니다.
- 교차로는 신호 없이 선착순, 한 번에 한 대가 2초 동안 지나갑니다. 유턴은 막다른 곳에서만.
- 경로는 자유 흐름 시간 기준 Dijkstra. 차선을 막으면(`set_lane_blocked`) 새 경로는 그 차선을 피하고, 이미 가던 차량은 교차로에서 경로를 다시 찾습니다.
- 결정론: 정수 ms 시간, (시각, 순번) 이벤트 순서, 시드 고정 난수(SplitMix64), 동률 처리 규칙 고정.
- 파라미터와 가정: [research-log.md](../docs/design/research-log.md#traffic-queue)

`maps/pyeongtaek-station`에서 차량 100대(시드 42, 5분 동안 출발): 모두 도착, 평균 이동 시간 139.6 초, 최장 361.8 초. 같은 시드로 두 번 돌리면 모든 차량 상태가 같습니다 (`crates/sim/tests/pyeongtaek_vehicles.rs`).

## 명령 (`Simulation::submit`)

`AddRoad`·`RemoveRoad`·`SetZone`을 [schema/commands](../schema/commands/README.md) 형식으로 받습니다. 다음 `step` 시작 때 넣은 순서대로 적용하고, 결과(`applied`/`rejected`)는 `Simulation::outcomes`에 남습니다.

- 도로 그래프는 노드·엣지·차선을 뒤에 붙이기만 하고, 없앤 엣지는 `removed`로 표시해 ID가 바뀌지 않습니다. `Traffic::sync`가 새 부분을 교통 모델에 반영합니다.
- 도로가 바뀌면 영향받는 차량의 남은 경로를 다시 찾습니다 (#5의 재탐색 재사용).
- 구역은 `World::zones`(terrain과 같은 셀 격자)에 저장합니다. 통근 수요 생성(#7)의 입력입니다.
- 평택역 맵에서 100대(시드 7)가 가장 많이 지나는 엣지를 출발 전에 없애면 26대가 경로를 바꾸고 100대 모두 도착합니다 (평균 149.0 초). 같은 명령을 두 번 돌리면 결과가 같습니다 (`crates/sim/tests/commands.rs`).
- 가정: [research-log.md](../docs/design/research-log.md#commands)

## 통근 수요 (`crates/sim/src/demand.rs`)

`Simulation::add_commute(params, seed)`가 `World::zones`에서 통근자를 만들고 출근·퇴근 차량 통행을 예약합니다. 주거 셀 → 업무·상업 셀(남은 일자리에 비례), 출근 08:00·퇴근 18:00 ± 30분. 파라미터는 모두 가정입니다 ([research-log.md](../docs/design/research-log.md#commute-demand)).

평택역 맵 예 (`crates/sim/tests/commute.rs`): 남서쪽 주거 169셀, 역 앞 업무 100셀, 동쪽 상업 100셀 → 통근자 1,690명, 통행 3,380개, 하루 안에 모두 도착(평균 190.2 초, 최장 416.6 초). 시간대별 도로 위 차량은 07시 802·08시 881, 17시 776·18시 900대이고 03시·12시는 0대로, 출퇴근 피크가 생깁니다.

## Python 바인딩 (`crates/py`)

`import dongne_core`로 `Simulation.load` → `submit`(명령) → `step` → `snapshot`(dict)을 씁니다. 사용법과 빌드: [crates/py/README.md](crates/py/README.md). 스냅샷 형식: [schema/snapshots](../schema/snapshots/README.md). libpython이 필요해 workspace `default-members`에서 빠져 있고, CI `python-binding` 잡이 maturin + pytest로 검사합니다.
