# 기여 가이드

## 1. 클린룸 규칙 (필수)

자세한 내용은 [docs/legal/clean-room.md](docs/legal/clean-room.md)를 보세요. 요약:

- 상용 게임(Cities: Skylines 시리즈 포함)의 코드, 에셋, 디컴파일 결과, 모드 SDK 내부 코드를 가져오지 않습니다.
- 리버스 엔지니어링으로 얻은 수치·공식을 쓰지 않습니다. 공개 논문·도서·오픈소스의 알고리즘은 출처를 밝히고 씁니다.
- 구글 지도·어스, 카카오맵, 네이버 지도 등 재배포가 허용되지 않는 지도의 데이터·스크린샷·트레이싱을 쓰지 않습니다.
- AI 코딩 에이전트가 쓴 코드도 같은 규칙을 따르며, 사람이 리뷰한 뒤 서명합니다.

## 2. 서명 (DCO)

모든 커밋에 `git commit -s`로 `Signed-off-by:` 줄을 넣어 주세요. 이는 [Developer Certificate of Origin](https://developercertificate.org/)에 동의한다는 뜻입니다.

## 3. 개발 환경

| 폴더 | 도구 | 확인 명령 |
| --- | --- | --- |
| `core/` | Rust stable | `cd core && cargo test` |
| `gis/` | Python 3.12, uv | `cd gis && uv run pytest` |
| `ai/` | Python 3.12, uv | `cd ai && uv run pytest` |
| `client/` | Godot 4 | Godot 에디터에서 `client/project.godot` 열기 |

## 4. PR 절차

1. 이슈를 먼저 열거나 기존 이슈에 댓글로 작업 의사를 남깁니다.
2. 한 PR은 되도록 한 폴더(한 팀) 안에서 끝냅니다. `schema/`를 바꾸면 관련 팀 모두의 리뷰가 필요합니다.
3. PR 템플릿의 클린룸 체크박스를 확인합니다.
4. 큰 설계 결정은 `docs/adr/`에 ADR로 남깁니다.

## 5. 라이선스

기여한 코드는 GPL-3.0-or-later, 아트·문서는 CC BY-SA 4.0, 맵 데이터는 ODbL 1.0으로 배포됩니다.
