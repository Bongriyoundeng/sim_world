# climate — 기후 시나리오 파일 (v0)

2단계 #38. 폭염·침수 모델(#39·#40)의 외부 입력입니다. 동네 정책은 시나리오를 바꾸지 못합니다([ADR 0003](../../docs/adr/0003-climate-system.md)). Rust 타입: `core/crates/api/src/climate.rs` (`ClimateScenario::validate`), 예시: [`examples/synthetic.json`](examples/synthetic.json) — api 테스트가 예시를 그대로 읽어 검증합니다.

| 필드 | 뜻 |
| --- | --- |
| `format_version` | 0 |
| `name`, `source` | 이름, 출처. **synthetic이면 "값을 임의로 정함"을 적어야 함** (비어 있으면 거부) |
| `ssp` | SSP 경로(예: `"SSP2-4.5"`), synthetic이면 `null` |
| `years[]` | 연도 오름차순. `summer_tmax_mean_c`·`summer_tmax_sd_c`(여름 일 최고기온 분포 °C), `heatwave_days`(일 최고 33 °C 이상인 날 수), `max_1h_rain_mm`(1시간 최대 강수량) |
| `events[]` | 특정 날 사건. `year`, `season`(0 봄·1 여름·2 가을·3 겨울, [ADR 0007](../../docs/adr/0007-time-model.md)), `rain_mm_per_h`(0~23시 24개, 없으면 빈 배열), `tmax_c`(선택), `source`(실측 관측소·기관 또는 "가정") |

- 연도 값이 없는 해는 가장 가까운 앞 연도 값을 씁니다.
- 계절 진행(ADR 0007)은 대표일 외에 그 계절의 사건일을 따로 돌립니다(#39·#40에서 연결).
- 실측 자료: 기상청 종관기상관측(ASOS) 시간 자료(ADR 0008, 출처 표시 필수). 공공데이터포털 API는 활용 승인 후 `gis`의 도구로 받습니다(2026-11-28 현재 HTTP 403 — 승인 대기).
