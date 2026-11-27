# 데이터 출처 정책

| 데이터 | 용도 | 라이선스 | 받는 곳 |
| --- | --- | --- | --- |
| OpenStreetMap | 도로, 건물, 토지 이용 | ODbL 1.0 | Overpass API, Geofabrik 추출본 |
| Copernicus DEM GLO-30 | 지형(30m 해상도) | Copernicus DEM 라이선스 (무료, 전 세계) | Copernicus Data Space / AWS Open Data |

## 규칙

- 원본 OSM·DEM 파일은 저장소에 커밋하지 않습니다. `tools/`의 스크립트로 내려받습니다.
- 변환된 맵 패키지는 `manifest.json`의 `attribution` 필드에 출처를 반드시 넣고, 없으면 CI가 실패합니다.
- `maps/`의 샘플은 ODbL 1.0 파생 데이터베이스로 배포합니다.
- 새 데이터 소스를 추가하려면 ADR을 먼저 작성하고 라이선스를 확인합니다.

표기 문구는 [ATTRIBUTION.md](../../ATTRIBUTION.md)를 보세요.
