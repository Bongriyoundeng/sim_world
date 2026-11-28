# 아키텍처

```
gis/ ──(맵 패키지)──▶ core/ ◀──(명령)── client/ (Godot, GDExtension)
                        │  ──(스냅샷·이벤트)──▶
                        │
                        ◀──(명령·페르소나 템플릿)── ai/ (Python, PyO3)
                           ──(통계·스냅샷)──▶
```

## 원칙

1. 코어는 화면을 모릅니다. 명령을 받아 틱을 돌리고 상태를 내보낼 뿐입니다.
2. 코어는 결정론적입니다. 고정 틱과 시드 고정 난수를 쓰고, 시뮬 상태는 정수·고정소수점으로 계산해 Linux·Windows·macOS에서 같은 결과를 냅니다. 규칙: [design/region-link.md](design/region-link.md#결정론-규칙-지금부터-코어에-적용)
3. LLM은 틱 안에서 부르지 않습니다. `ai/`가 만든 결과 파일을 코어가 읽습니다.
4. 동네 바깥 세계는 **경계 포트**로만 만납니다. 혼자 할 때는 가상 이웃이, 지역 연결(3단계 이후) 때는 이웃 플레이어가 포트를 채웁니다 ([ADR 0006](adr/0006-region-link.md)).
5. 좌표는 동네 중심을 원점으로 한 미터 단위 로컬 좌표입니다. 위경도 변환은 `gis/`가 한 번만 합니다.

## 코어 API (0단계)

| 함수 | 설명 |
| --- | --- |
| `load_map(path)` | 맵 패키지 폴더를 읽어 월드 생성 |
| `submit(command)` | 명령을 다음 틱 큐에 넣음 |
| `step(n_ticks)` | n틱 진행, 이벤트 목록 반환 |
| `snapshot(view)` | 필요한 만큼의 상태·통계 반환 |
| `state_hash()` | 결정론 상태 해시 (테스트·리플레이·검증) |

경계 포트 API(`boundary_gates`, `set_boundary_inputs`, `boundary_outputs`)는 [region-link.md](design/region-link.md#코어-api-초안-구현-담당이-다듬어도-됨)에 초안이 있습니다. 1단계에서는 `state_hash`만 먼저 만듭니다.

## 데이터 약속

모든 형식은 [`schema/`](../schema/)에 정의합니다. 맵 패키지 v0 형식은 [`schema/map-package/README.md`](../schema/map-package/README.md)를 보세요.
