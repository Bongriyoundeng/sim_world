# pyeongtaek-station

샘플 맵 패키지입니다. 근거: [docs/design/sample-area.md](../../docs/design/sample-area.md)

## 출처와 라이선스

이 폴더의 데이터는 OpenStreetMap에서 만든 파생 데이터베이스이며
[Open Database License (ODbL) 1.0](https://opendatacommons.org/licenses/odbl/1-0/)로 배포합니다.

© OpenStreetMap contributors — https://www.openstreetmap.org/copyright

## 범위

| 항목 | 값 |
| --- | --- |
| 원점 (origin) | 북위 36.993, 동경 127.087 |
| 크기 | 1500 m × 1500 m (origin 중심) |
| 받은 범위 (bbox) | S 36.986242, W 127.078574, N 36.999758, E 127.095426 |
| 좌표 | origin 중심 횡메르카토르, 미터 (x 동쪽, y 북쪽), 소수 2자리 |

## 내용

| 파일 | 내용 |
| --- | --- |
| `manifest.json` | 메타데이터. `terrain` 블록은 자리만 잡아 둔 값(고도 0~0)이고, `terrain.png`와 실제 고도 범위는 이슈 #2에서 채움 |
| `roads.geojson` | 도로 LineString 882개 (OSM way 862개, 범위 경계에서 자름) |
| `buildings.geojson` | 건물 Polygon 3048개 (무게중심이 범위 안인 건물, 자르지 않음) |

건물 층수(`building:levels`)가 있는 건물: 3개 (0.1%), 높이(`height`)가 있는 건물: 0개.

## 만든 방법

```sh
cd gis
uv run python -m dongne_gis build --name pyeongtaek-station --lat 36.993 --lon 127.087 --size 1500 --out ../maps/pyeongtaek-station
```

- 생성 일시: 2026-11-27 21:54 UTC
- OSM 데이터 기준 시각 (Overpass): 2026-11-27T21:51:46Z
