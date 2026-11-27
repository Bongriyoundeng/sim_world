<!-- claude-projects-attribution: {"github_login":"Bongriyoundeng"} -->
_Requested by **차승현** · [project thread](https://claude.ai/code/project/chan_01RUdFMD8ruNZvShzu9H1cAk?thread=cmsg_01RUdFMD8ruNZvShzu9H1cAk9XMaq6kC7M9cQAbLgnwTsn)_

## 요약

여러 플레이어의 동네를 잇는 "지역 연결"을 3단계 이후 로드맵에 넣고, 지금은 코어 설계에 **경계 포트**와 **플랫폼 간 결정론**만 반영합니다. 문서만 바꾸며 코드는 건드리지 않습니다.

- `docs/adr/0006-region-link.md`: 결정과 근거. 로컬 시뮬 + 서버는 경계 데이터만 중계, 계절 단위 비동기 교환, 가상 이웃, 결정론적 재시뮬 검증.
- `docs/design/region-link.md`: 경계 포트 표, 코어 API 초안(`boundary_gates`, `set_boundary_inputs`, `boundary_outputs`, `state_hash`), 결정론 규칙 6개, 재미 포인트, 열린 문제.
- `docs/roadmap.md`, `docs/architecture.md`, `docs/design/README.md`, `docs/design/research-log.md` 갱신.

## 근거

SimCity(2013) 항상 온라인 출시 실패(PC Gamer, Kotaku), 결정론 락스텝의 대기 문제(Gaffer On Games, Factorio FFF #76·#188), 부동소수점 결정론의 한계(Gaffer On Games). 자세한 내용은 research-log의 "지역 연결" 항목.

## 코어 작업에 주는 영향

이슈 #4·#5 담당: 경계 관문 노드를 일반 노드와 구분하고, 결정론 테스트에 `state_hash` 비교를 씁니다. 시뮬 상태는 정수·고정소수점으로 계산합니다.

🤖 Generated with [Claude Code](https://claude.com/claude-code)

https://claude.ai/code/session_01Hov7NEXA6UbWCgPng6fUjL
