# ai/ — AI 시뮬레이션 (Python)

| 모듈 | 역할 |
| --- | --- |
| `personas` | 대표 페르소나 수백 명의 생활 패턴 템플릿 생성 → `schema/personas` |
| `advisor`, `news` | 도시 통계를 읽고 시장 보좌관 설명·뉴스 생성 |
| `llm` | 공급사 어댑터 (클라우드 API, 로컬 모델) |
| `experiments/` | 화면 없이 코어를 돌리는 실험 스크립트 |
| `prompts/` | 프롬프트 템플릿 |

원칙: LLM은 틱 밖에서만 부르고, 코어는 그 결과 파일만 읽습니다. API 키는 환경변수로만 받습니다.

```sh
cd ai && uv run pytest
```
