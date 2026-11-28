# Python 바인딩 (PyO3 + maturin)

Python에서 `import dongne_core`로 코어를 불러 화면 없이 실험합니다 (`ai/experiments/` 등).

```sh
cd core/crates/py
uv venv --python 3.12 .venv
uv pip install --python .venv maturin pytest
.venv/Scripts/maturin develop --uv     # macOS·Linux: .venv/bin/maturin
.venv/Scripts/python -m pytest tests
```

```python
import dongne_core

sim = dongne_core.Simulation.load("maps/pyeongtaek-station")
sim.add_random_trips(100, seed=42, depart_window_s=300)
sim.submit({"type": "set_zone", "area": {"cells": [[10, 12]]}, "zone": "residential"})
sim.step(600)                      # 600틱 = 60초
snap = sim.snapshot()              # dict, schema/snapshots 형식
print(snap["traffic"])
```

| 메서드 | 뜻 |
| --- | --- |
| `Simulation.load(path)` | 맵 패키지 폴더 읽기 (없으면 `OSError`) |
| `step(n_ticks=1)` | n틱 진행 (쌓인 명령 먼저 적용) |
| `tick`, `time_s` | 진행한 틱 수, 시각(초) |
| `submit(command)` | dict 또는 JSON 문자열 명령 ([schema/commands](../../../schema/commands/README.md)), 형식이 틀리면 `ValueError` |
| `add_trip(origin, dest, depart_s=0)` | 통행 하나 예약 (경로 없으면 `None`) |
| `add_random_trips(n, seed=0, depart_window_s=300)` | 무작위 통행 n개 |
| `add_commute(seed=0, residents_per_cell=None, …)` | 구역 맵에서 출퇴근 통행 생성, 요약 dict |
| `run_day()`, `run_season(seed=0, residents_per_cell=None)`, `day` | 다음 자정까지 진행 / 한 계절 진행(ADR 0007 대표일 방식), 보고서 dict |
| `snapshot(vehicles=True)` | 현재 상태 dict ([schema/snapshots](../../../schema/snapshots/README.md)) |

- `abi3-py312`로 빌드해 Python 3.12 이상 어디서나 같은 wheel을 씁니다.
- 이 crate는 libpython이 필요해 workspace의 `default-members`에서 빠져 있습니다. `cd core && cargo test`는 이 crate를 빌드하지 않고, CI의 `python-binding` 잡이 maturin으로 빌드해 pytest를 돌립니다.
