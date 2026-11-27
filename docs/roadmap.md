# 로드맵

## 0단계 (1~2개월): 동네를 화면에 띄우기

- [x] 라이선스, 레포 구조 확정
- [x] 맵 패키지 v0 스키마
- [x] `gis/`: 샘플 동네 → 맵 패키지 변환 (#1·#2·#3)
- [x] `core/`: 맵 패키지 읽기, 도로 그래프 (#4)
- [x] `core/crates/py`: Python에서 코어 호출 (#8)
- [x] `client/`: 지형·도로·건물 표시 (#9)
- [x] CI: 폴더별 빌드·테스트, 스키마·출처 검사

완료 기준: 샘플 동네가 Godot 화면에 보이는 캡처 — [완료](images/pyeongtaek-station-map-view.png) (2026-11-27)

## 1단계 (3~4개월): 도로·구역·차량 데모

동네 한 곳에서 도로 건설, 구역 지정, 차량 이동이 되는 데모. 데모 GIF로 기여자를 모집합니다.

작업 목록: [design/phase1-backlog.md](design/phase1-backlog.md) — 이슈 #1~#14 모두 완료 (2026-11-27). 데모 GIF: [images/demo.gif](images/demo.gif). 밸런스와 남은 가정: [design/balance.md](design/balance.md)

## 2단계: 인구·경제·기후·AI 보좌관

- 인구·경제, 재정 지표
- 기후 변화: 폭염·집중호우 침수 격자 모델과 기후 정책 ([design/climate.md](design/climate.md), [ADR 0003](adr/0003-climate-system.md))
- AI 보좌관 브리핑

## 3단계 이후

- LLM 페르소나 확대, 모드 지원(해수면 상승 등 추가 위험)
- 지역 연결: 각자 자기 동네를 돌리고 계절마다 이웃 동네와 통근·화물·무역·인구 이동·하천 흐름을 주고받는 비동기 연결 ([design/region-link.md](design/region-link.md), [ADR 0006](adr/0006-region-link.md))

지역 연결을 위해 지금부터 지키는 것: 코어의 경계 포트(혼자 할 때는 가상 이웃이 채움)와 플랫폼 간 결정론.
