# pyeongtaek-station

샘플 맵 패키지입니다. 근거: [docs/design/sample-area.md](../../docs/design/sample-area.md)

## 출처와 라이선스

도로·건물·토지 피복(`roads.geojson`, `buildings.geojson`, `landcover.png`)은 OpenStreetMap에서 만든 파생 데이터베이스이며
[Open Database License (ODbL) 1.0](https://opendatacommons.org/licenses/odbl/1-0/)로 배포합니다.

© OpenStreetMap contributors — https://www.openstreetmap.org/copyright

지형(`terrain.png`)은 Copernicus DEM GLO-30에서 만들었습니다.

© DLR e.V. 2010-2014 and © Airbus Defence and Space GmbH 2014-2018 provided under COPERNICUS by the European Union and ESA; all rights reserved

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
| `manifest.json` | 메타데이터 (origin, 범위, 지형 고도 범위, 토지 피복 채널, 출처) |
| `terrain.png` | 16비트 그레이스케일 높이맵 50×50 (30m 셀, 행 0 = 북쪽). 픽셀 0~65535 = 고도 2.93~47.12 m. Copernicus DEM은 지표면 모델(DSM)이라 건물·나무 높이가 섞여 있음 |
| `landcover.png` | 8비트 RGB 토지 피복 (terrain과 같은 격자). R = 불투수, G = 녹지·나무 수관, B = 수면, 0~255 = 셀 면적 비율 0~1. OSM 도형에서 추정 (구역 평균: 불투수 44.5%, 녹지 4.0%, 수면 0.2%) |
| `roads.geojson` | 도로 LineString 882개 (OSM way 862개, 범위 경계에서 자름) |
| `buildings.geojson` | 건물 Polygon 3048개 (무게중심이 범위 안인 건물, 자르지 않음) |

건물 층수(`building:levels`)가 있는 건물: 3개 (0.1%), 높이(`height`)가 있는 건물: 0개.

## 만든 방법

```sh
cd gis
uv run python -m dongne_gis build --name pyeongtaek-station --lat 36.993 --lon 127.087 --size 1500 --out ../maps/pyeongtaek-station
```

- 생성 일시: 2026-11-27 22:04 UTC
- OSM 데이터 기준 시각 (Overpass): 2026-11-27T22:03:05Z
