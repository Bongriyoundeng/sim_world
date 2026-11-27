"""LLM 공급사 어댑터 인터페이스.

클라우드 API와 로컬 모델을 같은 인터페이스로 다룬다.
API 키는 환경변수로만 받고 저장소에 커밋하지 않는다.
LLM은 시뮬레이션 틱 안에서 호출하지 않는다.
"""

from __future__ import annotations

from typing import Protocol


class LLM(Protocol):
    def complete(self, system: str, prompt: str) -> str: ...


class EchoLLM:
    """테스트용 가짜 LLM. 프롬프트를 그대로 돌려준다."""

    def complete(self, system: str, prompt: str) -> str:
        return prompt
