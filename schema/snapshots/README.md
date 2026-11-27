# snapshots

코어 → 클라이언트·AI로 보내는 현재 상태입니다. 만드는 곳: `core/crates/sim/src/snapshot.rs`의 `Simulation::snapshot(include_vehicles)`. Python에서는 `dongne_core.Simulation.snapshot()`이 같은 내용을 dict로 돌려줍니다.

> 아직 Rust 타입이 아니라 JSON 값으로 만듭니다. 클라이언트 연결(#9)에서 필드가 정해지면 `core/crates/api`의 타입으로 옮기고 예시 JSON 테스트를 붙입니다.

## v0 형식

```json
{
  "format_version": 0,
  "map": "pyeongtaek-station",
  "tick": 6000,
  "time_s": 600.0,
  "traffic": { "vehicles": 100, "arrived": 100, "mean_travel_s": 139.6, "max_travel_s": 361.8 },
  "road_graph": { "nodes": 908, "edges": 1272, "lanes": 2588, "lane_km": 137.5 },
  "zones": 0,
  "outcomes": [
    { "id": 0, "tick": 0, "result": { "status": "applied", "cells": 2 } }
  ],
  "vehicles": [
    {
      "id": 0, "state": "arrived", "origin": 12, "dest": 345,
      "lane": null, "edge": null, "depart_s": 31.2, "arrive_s": 170.4
    }
  ]
}
```

| 필드 | 뜻 |
| --- | --- |
| `tick`, `time_s` | 진행한 틱 수(1틱 = 100 ms), 시뮬레이션 시각(초, 자정 = 0) |
| `traffic` | 예약된 차량 수, 도착한 수, 도착한 차량의 평균·최장 이동 시간 |
| `road_graph` | 없앤 도로를 뺀 노드·엣지·차선 수, 총 차선 길이 |
| `zones` | 구역이 지정된 셀 수 |
| `outcomes` | 지금까지 처리한 명령 결과 ([commands](../commands/README.md)) |
| `vehicles` | (`include_vehicles`일 때) 차량별 상태 `scheduled`·`waiting_to_enter`·`crossing`·`queued`·`waiting_to_advance`·`arrived`, 지금 차선과 엣지, 출발·도착 시각 |
