# tests/scenarios/ — 통합 시나리오 (QA)

화면 없이 코어를 돌려 "이 도로를 막으면 통행 시간이 늘어나야 한다" 같은 기대를 검사합니다. Python 바인딩(`dongne_core`)으로 돌립니다.

```sh
# core/crates/py/README.md대로 바인딩을 설치한 뒤, 저장소 루트에서
core/crates/py/.venv/Scripts/python -m pytest -q -s tests/scenarios   # macOS·Linux: .venv/bin/python
```

CI: `core` 워크플로의 `python-binding` 잡이 바인딩을 빌드한 뒤 이 폴더를 돌립니다.

| 파일 | 내용 |
| --- | --- |
| `scenario.py` | 공통 설정: 평택역 맵, 구역(남서쪽 주거·역 앞 업무·동쪽 상업), 출퇴근 하루 실행 (시드 2026). `ai/experiments`도 이 설정을 씁니다 |
| `test_road_closure.py` | 이슈 #12. 기준 하루에서 실제로 가장 많은 차량이 지난 도로를 출발 전에 막고, 모두 도착하는지·평균 통근 시간이 늘어나는지·결과가 결정론적인지 확인 |

## 결과 (2026-11-27 OSM 맵, 시드 2026)

| 항목 | 값 |
| --- | --- |
| 통근자 / 통행 | 1,690명 / 3,380개 |
| 막은 도로 | 엣지 421, tertiary 통복로 (기준 하루에 2,718대 통과) |
| 평균 통근 시간 | 190.2초 → **205.2초 (+15.1초)** |
