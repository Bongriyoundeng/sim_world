# commands

클라이언트·AI → 코어 명령입니다. Rust 타입은 `core/crates/api/src/command.rs`이고, JSON 예시는 [`examples/`](examples/)에 있습니다. 예시는 `core/crates/api`의 테스트가 그대로 읽어 왕복 직렬화를 검사하므로, 타입과 예시가 어긋나면 `cargo test`가 실패합니다.

> JSON Schema 파일 자동 생성은 의존성(schemars) 추가가 필요해 미뤘습니다. 지금은 예시 JSON + 테스트가 약속입니다.

모든 명령은 `type` 필드로 종류를 구분합니다.

| `type` | 필드 | 뜻 |
| --- | --- | --- |
| `add_road` | `from`, `to`: `{"node": 번호}` 또는 `{"point": {"x", "y"}}` · `highway` · `lanes`(선택, 양방향 합계) · `oneway`(선택, 기본 false) | 두 끝을 잇는 직선 도로. 좌표 끝은 5 m 안의 기존 노드에 붙고, 없으면 새 노드 |
| `remove_road` | `edge` | 도로 엣지를 없앰. 그 위의 차량은 끝까지 빠져나감 |
| `set_zone` | `area`: `{"cells": [[행, 열], …]}` 또는 `{"polygon": [{"x", "y"}, …]}` · `zone`: `residential`·`commercial`·`office`·`park` 또는 `null`(지우기) | 셀 단위 구역 지정. 격자는 terrain과 같음(행 0 = 북쪽). 폴리곤은 셀 중심이 안에 드는 셀 |

## 처리 규칙

- `submit`으로 넣은 명령은 다음 `step`이 시작될 때(지금 틱 경계) 넣은 순서대로 적용됩니다. 같은 입력이면 같은 결과가 나옵니다.
- 결과는 `CommandOutcome { id, tick, result }`로 남습니다. `result`는 `{"status": "applied", …}` 또는 `{"status": "rejected", "reason": "…"}`입니다. 거부된 명령은 상태를 바꾸지 않습니다.
- 도로가 바뀌면 아직 도착하지 않은 차량의 남은 경로를 다시 찾습니다 (추가: 모든 차량, 삭제: 그 도로를 지나갈 차량). `applied`의 `rerouted`가 경로가 바뀐 차량 수입니다.

## 거부 사유 예

- 없는 노드·엣지, 이미 없앤 엣지
- 차량 도로가 아닌 `highway`(footway 등), `lanes: 0`
- 맵 범위 밖 좌표, 두 끝이 같은 노드이거나 1 m보다 가까움
- 격자 밖 셀, 점이 3개 미만인 폴리곤, 칠할 셀이 없는 범위
