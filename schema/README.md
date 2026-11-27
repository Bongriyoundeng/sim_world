# schema/

코어·클라이언트·AI·GIS 사이의 데이터 약속입니다. 여기를 바꾸면 관련 팀 모두의 리뷰가 필요합니다.

| 폴더 | 내용 | 만드는 쪽 → 쓰는 쪽 |
| --- | --- | --- |
| `map-package/` | 맵 패키지 형식 | gis → core, client |
| `commands/` | 명령 (도로 건설, 구역 지정, 정책 변경) | client, ai → core |
| `snapshots/` | 스냅샷·이벤트 | core → client, ai |
| `personas/` | LLM 페르소나 템플릿 | ai → core |

JSON Schema 파일은 1단계부터 Rust 타입(`core/crates/api`)에서 자동 생성하고, CI가 어긋남을 검사합니다.
