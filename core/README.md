# core/ — 시뮬레이션 코어 (Rust)

| crate | 역할 |
| --- | --- |
| `crates/api` | 명령·스냅샷·맵 패키지 타입 (`schema/`와 일치) |
| `crates/world` | 맵 패키지 읽기, 도로 그래프, 지형 |
| `crates/sim` | 틱 루프, 교통·경제·인구 |
| `crates/py` | Python 바인딩 (PyO3) — 0단계 6번 작업에서 추가 |
| `crates/godot` | Godot 바인딩 (GDExtension) — 0단계 7번 작업에서 추가 |

```sh
cd core && cargo test
```
