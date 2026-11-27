# sim_world — 우리 동네 AI 도시 시뮬레이터

실제 동네를 불러와 정책을 바꾸면 AI 시민이 어떻게 반응하는지 보는 오픈소스 도시 시뮬레이터입니다.
그래픽과 규모로 경쟁하지 않고, "우리 동네 + 정책 + AI 시민 반응" 하나에 집중합니다.

> 현재 0단계(레포 구조·데이터 파이프라인)입니다. 아직 플레이할 수 있는 빌드는 없습니다.

## 구조

| 폴더 | 담당 | 내용 |
| --- | --- | --- |
| `docs/` | 리드/PM, 게임 디자인 | 비전, 로드맵, 설계 결정(ADR), 법적 규칙 |
| `schema/` | 전 팀 | 코어·클라이언트·AI 사이의 데이터 약속 |
| `core/` | 시뮬레이션 코어 | Rust. 틱 루프, 도로 그래프, 교통·경제·인구 |
| `gis/` | GIS | Python. OSM + Copernicus DEM → 맵 패키지 |
| `ai/` | AI 시뮬레이션 | Python. 페르소나, 보좌관, 화면 없는 실험 |
| `client/` | 클라이언트 | Godot 4. 화면과 UI |
| `art/` | 아트 | 절차 생성 규칙, 자체 제작 에셋 |
| `maps/` | GIS | 작은 샘플 맵 패키지 (ODbL) |
| `tests/scenarios/` | QA | 화면 없는 통합 시나리오 |
| `tools/` | 공용 | 빌드·데이터 내려받기 스크립트 |

전체 구조와 인터페이스는 [docs/architecture.md](docs/architecture.md)에 있습니다.

## 기여하기

기여 전에 [CONTRIBUTING.md](CONTRIBUTING.md)와 [클린룸 규칙](docs/legal/clean-room.md)을 꼭 읽어 주세요.
상용 게임의 코드·에셋 반입과 재배포가 허용되지 않는 지도 데이터 사용은 금지입니다.

## 라이선스와 출처

- 코드: [GPL-3.0-or-later](LICENSE)
- 아트 에셋·문서: CC BY-SA 4.0
- `maps/`의 지도 데이터: ODbL 1.0
- 데이터 출처 표기: [ATTRIBUTION.md](ATTRIBUTION.md)
