# 근거 기록 (Research Log)

기획 결정마다 근거 자료를 여기 남깁니다. 각 항목은 직접 열어 확인한 페이지만 적습니다. 조사일: 2026-11-27.

## 기후 위험의 크기 (한국)

<a id="heat-korea"></a>
**폭염 — 온열질환 감시 결과**
- 2024-05-20 ~ 08-19 누적 온열질환자 2,890명, 추정 사망 26명. 2011년 감시 시작 이래 두 번째. 최다는 2018년 4,526명(사망 48명). 남성 77.2%, 실외 발생 78.5%. 출처: 질병관리청 응급실감시체계, [주간경향 2024-08-20](https://weekly.khan.co.kr/article/202408201745001)
- 쓰인 곳: 위험 우선순위(ADR 0003), 실외 노동자·고령자 가중치(climate.md)

<a id="flood-seoul-2022"></a>
**침수 — 2022년 8월 서울 집중호우**
- 동작구 신대방동 1시간 141.5mm(기상청 지역 관측, 서울 공식값은 종로구 송월동). 서울 일 강수 381.5mm로 1907년 관측 이래 최고. 관악구 반지하 주택에서 3명 사망. 이재민 391명, 주택·상가 침수 741건. 출처: [Korea Herald via ANN, 2022-08-10](https://asianews.network/heaviest-rainfall-in-115-years-wreaks-havoc-on-greater-seoul)
- 쓰인 곳: 위험 우선순위, 반지하 가구 피해 가중치

## 열섬 완화 효과

<a id="heat-trees"></a>
**Kumar et al. (2024)** "Urban heat mitigation by green and blue infrastructure: Drivers, effectiveness, and future needs." *The Innovation* 5(2):100588. [PMC10909648](https://pmc.ncbi.nlm.nih.gov/articles/PMC10909648/)
- 관측된 최대 기온 저감: 식물원 5.0 ± 3.5 °C, 습지 4.9 ± 3.2 °C, 녹색 벽 4.1 ± 4.2 °C, 가로수 3.8 ± 3.1 °C.
- 본문 인용: 가로수 주간 기온 최대 1.5 °C 저감(Coutts et al.).
- 쓰인 곳: 냉각 기본값. 최대값이 아닌 보수적 값(1.0~1.5 °C)을 기본으로 택함 — 게임이 효과를 과장하지 않게.

**Li et al. (2024)** "Cooling efficacy of trees across cities is determined by background climate, urban morphology, and tree trait." *Communications Earth & Environment* 5:754. [PDF](https://heathealth.info/wp-content/uploads/s43247-024-01908-4.pdf)
- 보행자 높이 최대 12 °C 저감. 상록·낙엽 혼합이 단일 수종보다 약 0.5 °C 더 냉각.
- 쓰인 곳: 혼합 수종 보너스(+0.5 °C), 효과가 배경 기후·도시 형태에 따라 달라진다는 점 → 동네마다 계수 보정 여지.

## 침수 유출

<a id="runoff"></a>
**Iowa DOT Design Manual 4A-5, Rational Method** [링크](https://iowadot.gov/design-manual/chapter-4-drainage/4a-5-using-rational-method-determine-peak-flow)
- Q = CIA. 유출계수(5년 빈도): 포장·건물 0.94, 상업 0.81, 연립 주거 0.66, 단독 주거 0.48, 잔디 0.22, 공원 0.21.
- 쓰인 곳: 침수 모델 1단계. 한국 기준 값으로 교체 가능한지는 조사 과제.

## 게임 디자인

<a id="serious-games"></a>
**Neset et al. (2020)** "Serious Gaming for Climate Adaptation—Assessing the Potential and Challenges of a Digital Serious Game for Urban Climate Adaptation." *Sustainability* 12(5):1789. [링크](https://www.mdpi.com/2071-1050/12/5/1789/htm)
- 게임이 적응 의사결정의 어려움을 성찰하게 하지만, 복잡도가 높으면 개별 조치의 결과를 파악하기 어렵다. 학습은 대부분 디브리핑에서 일어난다.
- 쓰인 곳: 루프의 "보좌관 브리핑" 단계, 기후 위험 2개로 제한, 지표 5개 상한.

## 교통 시뮬레이션 방식 (1단계)

<a id="traffic-model"></a>
**A/B Street — Discrete event traffic simulation** (Dustin Carlino, 2021) [링크](https://a-b-street.github.io/docs/tech/trafficsim/discrete_event/index.html)
- 0.1초마다 모든 차량을 갱신하는 방식은 느리고 부동소수점 예외가 많았고, 부드러운 가감속은 핵심 아이디어에 가치를 더하지 않아 이벤트 방식(우선순위 큐)으로 바꿨다. 차량은 차선별 대기열로 움직이고, 충돌은 교차로에서 처리.
- 대가: 가감속·안전거리가 없음, 차선 변경은 주로 교차로에서.

**FHWA Traffic Analysis Toolbox Vol. III, Appendix A** [링크](https://ops.fhwa.dot.gov/trafficanalysistools/tat_vol3/sectapp_a.htm)
- 미시 모델은 개별 차량 궤적을 내지만 보정할 파라미터가 많고, 중시 모델은 개별 차량을 집계 관계(속도-교통량)로 움직이며, 거시 모델은 혼잡의 범위를 보는 데 적합. 이벤트 방식은 상태 변화가 드문 시스템에 적합.
- 쓰인 곳: 1단계 교통 모델 선택(ADR 0004).

## 데이터 라이선스

<a id="kogl"></a>
**공공누리 제1유형** [문화체육관광부](https://www.mcst.go.kr/site/s_open/kogl/koglType.jsp): 출처 표시, 상업적 이용 가능, 변형·2차적 저작물 작성 가능. 기상청 저작물은 제1유형 표시 대상([기상청 저작권 정책](https://www.kma.go.kr/kma/guide/copyright.jsp)) — 단, 개별 자료에 붙은 유형 표시를 확인할 것.

<a id="cds"></a>
**Copernicus Climate Data Store**: 2025-07-02부터 "Licence to use Copernicus Products"가 CC-BY 4.0으로 대체됨([ECMWF 포럼](https://forum.ecmwf.int/t/cc-by-licence-to-replace-licence-to-use-copernicus-products-on-02-july-2025/13464)). CMIP6 원자료는 파일의 `license` 속성을 따라야 하며 일부 모델은 비상업(CC BY-NC-SA) → 사용 불가([CMIP6 이용 조건](https://ewds.climate.copernicus.eu/licences/cmip6-wps)).

## 2단계 데이터 출처 확인 (2026-11-28)

<a id="kma-license"></a>
**기상청 기상자료개방포털** — 직접 열어 확인
- [종관기상관측(ASOS)](https://data.kma.go.kr/data/grnd/selectAsosRltmList.do?pgmNo=36): 분·시간·일·월·연 자료, 제공 지점 105개, 1904년~(지점·요소별 다름). 시간 자료는 한 번에 1년까지 조회. 그 밖의 지점은 방재기상관측(AWS).
- [저작권 보호 및 정책](https://data.kma.go.kr/cmmn/static/staticPage.do?page=pageCr): 기상청 저작물은 별도 허락 없이 자유 이용 가능, 공공누리 제1유형 표시 자료는 자유 이용, **출처를 구체적으로 표시해야 함**. 2~4유형 자료는 각 제한 확인, 공공누리 표시가 없는 자료는 사전 협의, 기상청이 저작권을 모두 갖지 않은 일부 자료는 허락 없이 변형·재배포 불가. 상업 이용 여부는 이 페이지에 명시 없음(공공누리 유형 정의는 kogl.or.kr에서 확인 필요).
- 쓰일 곳: 2단계 이슈 7(기후 입력).

**확인하지 못한 것** (2단계 이슈 4에서 다시 확인)
- 통계지리정보서비스(SGIS) Open API: [개발자 페이지](https://sgis.mods.go.kr/developer/html/main.html)에서 인구·가구·주택 센서스 통계 API가 있음을 확인. 가입 시 API 이용약관 동의가 필요하다고만 있고 약관 본문·공간 단위는 페이지에 없음. (sgis.kostat.go.kr → sgis.mods.go.kr로 주소가 바뀜)
- KOSIS 주민등록인구 표: 로그인 페이지로 넘어가 확인 못 함. 저작권 정책 페이지 주소(kosis.kr/serviceInfo/copyrightPolicy.do)는 404.
- 공공데이터포털 건축물대장·개별공시지가: 기존에 알려진 데이터 번호 주소가 404. 현재 주소를 찾아야 함.

## 교차로 신호 (2단계 #34)

<a id="signals"></a>
**FHWA Traffic Signal Timing Manual** (FHWA-HOP-08-024, 미국 연방도로청, 공공 저작물. 페이지 머리에 개정 2판으로 대체되었다는 안내가 있음) — 직접 열어 확인 (2026-11-28)
- [3장](https://ops.fhwa.dot.gov/publications/fhwahop08024/chapter3.htm) 3.3.2: 포화 교통류율은 흔히 차로당 시간당 1,500~2,000 승용차, 이상값은 보통 1,900. 3.4.3 실무 근사: 차로당 1,800대/시 → 차량당 녹색 2초. 3.3.1 출발 손실 약 2초. 3.3.3: HCM 기본 총 손실 시간은 현시당 4초.
- 같은 3.4.3 표 3-4 계획 단계 주기: 양쪽 도로 비보호 좌회전 60초, 한쪽 보호 좌회전 또는 분리 현시 90초, 양쪽 120초. (조정 운영 요구는 반영 안 된 값)
- [5장](https://ops.fhwa.dot.gov/publications/fhwahop08024/chapter5.htm) 5.3.2: 황색은 대략 3~6초(MUTCD), 전적색은 6초를 넘지 않음.
- 쓰인 곳: `core/crates/sim/src/traffic.rs`의 `SATURATION_HEADWAY_MS`(2초), `SIGNAL_CYCLE_MS`(60초), `LOST_TIME_PER_PHASE_MS`(4초).
- **가정**: 신호 위치 규칙(간선끼리 만나는 3갈래 이상 교차로) — OSM 평택역 범위에 `highway=traffic_signals`·`crossing=traffic_signals` 노드가 0개(2026-11-28 Overpass 확인)라 실제 위치를 쓸 수 없음. 최소 녹색 10초, 교차로별 신호 어긋남(노드 번호 × 7.919초), 교착 해소 60·300초, 국내 기준(도로용량편람) 값 확인은 조사 과제.

<a id="phase2-data"></a>
**2단계 인구·건물·지가 자료 (이슈 #35에서 다시 확인, 2026-11-28)** — 결정은 [ADR 0008](../adr/0008-phase2-data-sources.md)
- 공공데이터포털 [15097972](https://www.data.go.kr/data/15097972/fileData.do) 행정안전부 지역별(행정동) 성별 연령별 주민등록 인구수: CSV, 월간, 열은 행정기관코드·기준연월·시도·시군구·읍면동·계·남·여·연령별(0~110세 이상). 세대 수는 없음. 이용허락범위 제한 없음.
- [15134735](https://www.data.go.kr/data/15134735/openapi.do) 국토교통부 건축HUB 건축물대장정보: REST OpenAPI(JSON/XML), 서비스 키 필요, 월간, 기본개요·표제부·층별개요 등. 좌표 없음(지번 키). 이용허락범위 제한 없음. 세대 수·주용도 필드명은 샘플 응답으로 확인 필요.
- [15052266](https://www.data.go.kr/data/15052266/fileData.do) 국토교통부 개별공시지가정보: 필지 단위 원/㎡, 반기, 상세 페이지는 SHP·CSV, 목록은 CSV로 표기가 다름. 이용허락범위 제한 없음.
- SGIS [이용약관](https://sgis.mods.go.kr/view/member/clause) 제15조③(사전 승낙 없는 복제·재배포 금지)·제16조(승인 없는 상업 이용 금지)와 [저작권 정책](https://sgis.mods.go.kr/jsp/member/copyright.jsp)(공공누리 표시 저작물은 유형별 자유 이용)이 다르게 읽힘 → 보류.

## 건물 → 인구·종사자·통행 (2단계 #36)

<a id="land-use"></a>
코드: `core/crates/world/src/land_use.rs`, `core/crates/sim/src/building_demand.rs`. 직접 확인한 근거와 가정을 나눕니다 (2026-11-28).

**근거로 쓴 것**
- 방식: 국토교통부 「교통영향평가지침」(국토교통부고시, [국가법령정보센터](https://www.law.go.kr/행정규칙/교통영향평가지침)) 제2조 6호는 "교통유발 원단위"를 건축물 **용도별** 일정 단위시간(일반적으로 24시간)과 단위지표(단위인구·단위면적 등)로 통행을 나타낸 값으로 정의하고, 제12조는 원단위를 해당 지역 같은 용도·비슷한 규모 건축물의 실측·관련 자료로 산출하라고 함. → 용도별 "연면적 1,000㎡당 하루 통행" 방식을 따름. **지침 본문에 숫자 표는 없음**(실측·DB 자료로 산출).
- 종사자 1인당 연면적: 미국 EIA [CBECS 2012 표 PBA1](https://www.eia.gov/consumption/commercial/data/2012/bc/cfm/pba1.php) "Mean square feet per worker" — 사무(Office) 473 ft² ≈ 43.9㎡, 상업(Mercantile) 1,243 ft² ≈ 115.5㎡. 미국 자료라 국내 값으로 바꿀 과제.
- 인구 비교 기준: 행정안전부 행정동 주민등록 인구 2026-10-31 ([ADR 0008](../adr/0008-phase2-data-sources.md)) — 통복동 6,328명, 비전1동 48,768명 등.

**가정 (근거를 찾지 못함 — 확인 과제)**
- 1인당 주거면적 30㎡ (국토교통부 주거실태조사 결과 페이지를 열지 못함).
- ~~용도 불명 건물은 원도심 상가주택으로 보고 1층 상업 + 나머지 주거~~ → **건축물대장 통계로 교체** (#36 2차): 용도 불명 건물(`building=yes`, 평택역 맵 3,048개 중 2,946개)은 층수 태그가 없으면 2.11층, 연면적 중 주거 34.4% — 평택동·통복동·군문동 표제부의 단독주택·근린생활시설 2,544동 통계([building-register](#building-register)). 건물별 실제 값이 아니라 동 단위 평균이라는 한계가 남음(좌표 없음).
- 공공·교육·산업 종사자 1인당 100㎡, 거주자 중 통근 취업자 50%, 승용차 비율 50%(#37에서 밀도에 따라 바꿈).
- 방문 원단위(연면적 1,000㎡당 하루 차량 통행): 상업 20, 공공 5, 사무 2. 방문 도착 14시 ± 3시간(8~21시), 머무는 시간 1시간.
- OSM 행정동 경계(admin_level 7~10)가 평택역 범위에 없어(2026-11-28 Overpass 확인) 동 인구를 셀로 나눠 맞추지 못하고 밀도만 비교함.

<a id="building-register"></a>
**건축물대장 표제부 — 평택역 주변 법정동 통계 (2026-11-28 조회)** ([ADR 0008](../adr/0008-phase2-data-sources.md))
- 출처: 국토교통부 「건축HUB 건축물대장정보 서비스」 getBrTitleInfo, 공공데이터포털(이용허락범위 제한 없음). 관리자가 발급한 서비스 키(환경변수 `DATA_GO_KR_KEY`)로 조회, 키·요청 URL은 저장하지 않음. 원본(골라낸 필드)은 `data-raw/building-register/`(커밋 안 함). 도구: `cd gis && uv run python -m dongne_gis register --sigungu 41220 --bjdong 11300 11400 11500`.
- 범위: 평택시(41220) 법정동 평택동(11300) 1,161건, 통복동(11400) 1,676건, 군문동(11500) 177건 = 3,014건 (같은 범위 OSM 건물 3,048개와 비슷한 규모). 법정동 코드는 같은 API로 11300·11400·11500 등을 조회해 주소(`platPlc`)로 확인.
- **좌표가 없어 OSM 건물과 하나씩 맞출 수 없음.** 필지 경계(연속지적도)나 건물 도형(GIS건물통합정보)이 있어야 매칭 가능 — 공공데이터포털의 개별공시지가 WFS(15124014)는 외부 링크(제공형태 LINK)라 이 키로 쓸 수 없음. 그래서 지금은 법정동 단위 통계로만 씀.

| 주용도 | 건물 | 연면적 합(㎡) | 평균 지상층수 | 세대·가구·호 합 |
| --- | ---: | ---: | ---: | ---: |
| 단독주택 | 1,355 | 237,369 | 1.95 | 3,178 |
| 공동주택 | 214 | 7,327,826 | 8.19 | 7,734 |
| 제1종근린생활시설 | 705 | 252,949 | 2.2 | 139 |
| 제2종근린생활시설 | 484 | 200,129 | 2.43 | 160 |
| 판매시설 | 8 | 120,638 | 4.38 | 479 |
| 업무시설 | 31 | 72,345 | 5.32 | 532 |
| 교육연구시설 | 11 | 20,649 | 2.91 | 8 |

- 작은 건물(단독주택 + 제1·2종 근린생활시설) 2,544개의 평균 지상층수 **2.11층**, 연면적 중 주거(단독주택) 비율 **0.34**. → 1단계·#36의 "용도 불명 = 3층 상가주택(1층 상업 + 나머지 주거)" 가정보다 낮고 상업 비율이 큼.
- 공동주택 연면적 합(약 7.3백만 ㎡)은 세대 수(7,734)에 비해 지나치게 커서(세대당 수백 ㎡) 단지 전체 면적이 동마다 반복 기록됐을 가능성 → 이 값은 쓰지 않고 세대 수만 참고. 확인 과제.

## 지형 데이터 받는 곳

<a id="copernicus-dem-aws"></a>
**Copernicus DEM GLO-30 — AWS Open Data 버킷** (조사일 2026-11-27, 직접 열어 확인)
- 버킷 안내: [copernicus-dem-30m readme](https://copernicus-dem-30m.s3.amazonaws.com/readme.html). Copernicus DEM은 건물·인프라·식생을 포함한 지표면 모델(DSM)이고, Cloud Optimized GeoTIFF로 제공. 바다는 타일이 없고 고도 0으로 간주.
- 경로 규칙: `Copernicus_DSM_COG_[해상도]_[북위]_[동경]_DEM/` 폴더에 같은 이름의 `.tif`. 해상도는 각초 단위(GLO-30은 `10`), 북위·동경은 `N36_00`, `E127_00`처럼 소수부가 항상 `00`인 타일 남서쪽 정수 좌표.
- 실제 확인: [버킷 목록 (prefix N36_00_E127)](https://copernicus-dem-30m.s3.amazonaws.com/?list-type=2&prefix=Copernicus_DSM_COG_10_N36_00_E127&max-keys=10)에 `Copernicus_DSM_COG_10_N36_00_E127_00_DEM/` 폴더가 있고, [타일 파일](https://copernicus-dem-30m.s3.amazonaws.com/Copernicus_DSM_COG_10_N36_00_E127_00_DEM/Copernicus_DSM_COG_10_N36_00_E127_00_DEM.tif)이 HTTP 200, 약 49.6MB.
- 위도 0~50°의 GLO-30 타일은 3600×3600 픽셀(원본 3601에서 동·남쪽 공유 행·열 제거), DEFLATE 압축.
- 라이선스: 버킷 안내는 Copernicus Programme의 이용 조건(무료, 일반 공개)을 가리킴. 표기 문구는 [ATTRIBUTION.md](../../ATTRIBUTION.md)를 따름.
- 쓰인 곳: `gis/src/dongne_gis/terrain.py`, 이슈 #2

## 토지 피복 추정 (도로 폭)

<a id="road-width"></a>
**도로의 구조·시설 기준에 관한 규칙** (국토교통부령 제1588호, 2026-06-03 시행) [국가법령정보센터](https://www.law.go.kr/법령/도로의구조ㆍ시설기준에관한규칙) — 본문을 직접 열어 확인 (조사일 2026-11-27)
- 제10조 ③ 차로의 최소 폭 (도시지역): 설계속도 100 km/h 이상 3.50 m, 80 이상 3.25 m, 70 이상 3.25 m, 60 이상 3.00 m, 60 미만 3.00 m. ④ 설계속도 40 km/h 이하 도시지역 차로는 불가피하면 2.75 m 이상.
- 제16조 ③ 보도의 유효폭은 최소 2 m 이상 (지방지역 도로와 도시지역 국지도로는 불가피하면 1.5 m 이상).
- 쓰인 곳: `gis/src/dongne_gis/landcover.py`. 차로 폭 3.00 m(원도심 도로를 설계속도 80 km/h 미만으로 봄), motorway·trunk 3.25 m, footway 2.0 m. 이 값은 "최소" 폭이므로 실제 포장 면적보다 작게 추정될 수 있음 (보수적).
- **가정 (근거 없음)**: `lanes` 태그가 없을 때 유형별 차로 수, pedestrian 4 m, cycleway·path 1.5 m, track 3 m, 나무 점 수관 반경 3 m, 가로수 줄 폭 6 m, 하천 선 폭(river 20 m, stream 8 m, canal 8 m, drain 2 m, ditch 1.5 m). 측정 자료를 찾으면 교체.

## 도로 그래프 기본값 (1단계 교통)

<a id="road-lanes"></a>
**가정 (근거 없음)** — `core/crates/world/src/road_graph.rs`. 측정·법령 근거를 찾으면 교체합니다.
- `lanes` 태그가 없을 때 양방향 합계 차선 수: motorway·trunk·primary 4, secondary·tertiary·unclassified·residential 2, `*_link`·living_street·service 1. 토지 피복의 도로 폭(`gis/src/dongne_gis/landcover.py`)과 같은 값.
- 양방향 도로는 차선을 반씩 나누고 방향별 최소 1 (1차로 service·living_street도 양방향 통행으로 봄).
- 대기열 용량용 차량 간격 7.5 m (승용차 길이 + 정지 간격).
- 자유 흐름 속도 (km/h): motorway 100, trunk 80, primary 60, secondary·tertiary 50, unclassified·residential 30, living_street·service 20. 맵 패키지에 `maxspeed`가 들어오면 그 값을 우선하도록 바꿀 예정.
- 참고: 차로 **폭**은 법령 근거가 있음 ([road-width](#road-width)). 차로 **수**와 속도는 아직 없음.

## 차량 이동 (차선 대기열, 1단계)

<a id="traffic-queue"></a>
**가정 (근거 없음)** — `core/crates/sim/src/traffic.rs`. ADR 0004(이벤트 기반, 차선 대기열, 가감속 없음)를 따르는 구현의 파라미터. 근거를 찾으면 교체합니다.
- 시간 단위: 정수 밀리초, 틱 하나 = 100 ms. 이벤트는 (시각, 생성 순번) 순으로 처리.
- 교차로: 신호 없음, 선착순(요청 순서), 교차로 하나에 한 번에 한 대, 한 대 통과 2 초. 앞 차량의 다음 차선이 가득 차 있으면 그 차량은 건너뛰고 다음 요청을 본다(교차로 전체가 막히지 않게).
- 차선 주행 시간 = 엣지 길이 ÷ 자유 흐름 속도([road-lanes](#road-lanes)). 차선 안에서는 추월 없이 들어온 순서대로 나감.
- 차선 선택: 막히지 않고 자리가 있는 차선 중 차량이 가장 적은 차선, 동률이면 번호가 작은 차선.
- 회전 연결: 노드에 들어온 arc에서 나가는 모든 arc로 갈 수 있음. 유턴(같은 엣지 역방향)은 그 노드에서 다른 길이 없을 때(막다른 곳)만 허용.
- 경로: 자유 흐름 시간 + 교차로 통과 시간 기준 Dijkstra, 동률이면 arc 번호가 작은 쪽. 다음 arc의 차선이 모두 막히면 그 교차로에서 경로를 다시 찾음.
- 무작위 통행: 도로 그래프의 가장 큰 강연결 요소 안에서 출발·도착 노드를 SplitMix64(시드 고정)로 고르고, 출발 시각은 5분 구간에 고르게.

## 명령 처리 (도로·구역)

<a id="commands"></a>
**가정 (근거 없음)** — `core/crates/sim/src/lib.rs`, `schema/commands/README.md`.
- 도로 추가 명령의 좌표 끝은 반경 5 m 안의 가장 가까운 노드에 붙인다. 기존 도로 중간(노드가 아닌 곳)에 붙여 도로를 나누는 기능은 아직 없음.
- 새 도로 최소 길이 1 m. 새 도로는 직선이고 OSM id는 0.
- 없앤 도로 위의 차량은 그 차선 끝까지 빠져나간 뒤 다음 교차로에서 새 경로를 따른다.
- 도로를 추가하면 도착 전 모든 차량, 없애면 그 도로를 남은 경로에 가진 차량만 다음 결정 지점(지금 차선의 끝 노드)부터 경로를 다시 찾는다. 새 경로가 없으면 원래 경로를 두고 교차로에서 다시 시도.
- 명령은 다음 틱 경계에서 넣은 순서대로 적용.

## 통근 수요 (구역 → 출퇴근)

<a id="commute-demand"></a>
**가정 (근거 없음)** — `core/crates/sim/src/demand.rs` `DemandParams::default()`. 통계 자료(통근 시각 분포, 셀당 인구·일자리 밀도)를 찾으면 교체합니다.
- 주거 셀(30 m × 30 m)당 통근자 10명, 업무 셀당 일자리 30개, 상업 셀당 일자리 10개. 일자리가 모자라면 남는 주민은 통근하지 않음.
- 일터 선택: 남은 일자리 수에 비례한 확률 (거리 감쇠 없음 — 1.5 km 범위라 거리 효과는 2단계에서 검토).
- 출근 시각 평균 08:00, 퇴근 평균 18:00, 표준편차 각 30분인 정규분포 (균등분포 12개 합으로 근사, 시드 고정). 시뮬레이션 0 ms = 자정.
- 집·일터 셀은 셀 중심에서 300 m 안의 가장 가까운 도로 노드(가장 큰 강연결 요소 안)에 붙음. 없으면 그 셀은 빠짐.
- 1단계는 출근·퇴근 통행만. 쇼핑·등하교 등 다른 통행은 없음. 집과 일터가 같은 노드면 차량 통행을 만들지 않음.

## 건물 표시 높이

<a id="building-height"></a>
**가정 (근거 없음)** — `core/crates/world/src/buildings.rs`. 평택역 맵 건물 3,048개 중 `building:levels`가 있는 건물은 3개, `height`는 0개라 거의 모두 기본값을 씁니다.
- 순서: `height` 태그 → `building:levels` × 3 m → 유형별 기본값.
- 기본값: apartments 15층(45 m), house·detached·residential 2층(6 m), commercial·retail·office·school·public·hospital 4층(12 m), train_station·industrial·warehouse 6 m, roof·shed·garage·kiosk·construction 3 m, 그 밖(building=yes 등, 원도심 저층 상가주택으로 봄) 3층(9 m).
- 지금은 표시용입니다. 인구·일자리 추정에 쓰려면 근거(건축물대장 등 공개 자료와 라이선스)를 먼저 확인합니다 (#13).

## 건물 외형 (클라이언트 표시)

<a id="building-look"></a>
**표시 규칙 — 대부분 가정** — `client/scripts/buildings.gd`, `client/shaders/building.gdshader` (#55). 조사일: 2026-11-28.
- 평택역 맵 건물 3,048개 중 2,944개(96.6%)가 `building=yes`라 OSM 값만으로는 용도를 나눌 수 없습니다. `yes`는 바닥 면적으로 나눕니다 (**가정**): 100 m² 미만 단독주택, 100~700 m² 상가주택(1층 상점 + 위층 주거), 700 m² 초과 상업. 면적 분포: 10% 60 m², 중앙값 121 m², 90% 294 m², 99% 860 m².
- 그 밖의 값: apartments → 아파트, house·detached → 단독주택, commercial·retail·office → 상업, school·public·hospital 등 → 공공·학교, industrial·warehouse → 공장·창고, train_station → 역, roof·parking·construction 등 → 창 없는 부속 건물.
- 창문은 텍스처 파일 없이 셰이더로 그립니다: 층 높이 3 m(코어 `STOREY_M`과 같음), 창 간격 아파트 3.3 m·상가주택 3.0 m·단독주택 3.5 m·상업 2.6 m·공공 3.6 m·공장 6 m (**가정**, 실측 아님). 벽 한 변의 칸 수는 변 길이 ÷ 간격을 반올림해 창이 모서리에 걸리지 않게 합니다. 아파트의 짧은 옆벽(가장 긴 변의 45% 미만)은 창 없는 측벽으로 둡니다.
- 평지붕은 난간(0.9 m, 아파트 1.2 m)을 두르고, 단독주택·상가주택 평지붕의 60%를 녹색으로 칠합니다. 근거: 한국 옥상의 녹색은 산화크롬 안료가 든 우레탄 방수 도료 때문이고, 기본색이라 값이 싸다는 설명 ([Shuttle Delivery 블로그, 2024-01-18](https://partners.shuttledelivery.co.kr/blog-posts/why-are-so-many-roofs-in-south-korea-green), 출처를 밝히지 않은 상업 블로그라 2차 근거). 60%라는 비율은 **가정**입니다.
- 평지붕 일부(약 8%)는 흰색 쿨루프입니다. 서울시는 2014년 흰색 우레탄 방수 도료로 '옥상흰빛 캠페인'을 시작했고 ([서울시 환경 누리집](https://news.seoul.go.kr/env/archives/43080)), 2026년 취약가구 쿨루프 사업은 152개 건물(31,204 m²)에 칠했다고 보도됐습니다 ([Korea Times, 2026-09-16](https://www.koreatimes.co.kr/southkorea/society/20260916/heat-reflective-cool-roof-paint-cuts-summer-power-bills-for-vulnerable-seoul-households)). 비율은 **가정**이며, 2단계 기후 정책(쿨루프)이 생기면 정책 결과로 바뀌게 할 수 있습니다.
- 경사지붕은 꼭짓점 4개의 볼록한 작은 건물(260 m² 이하, 12 m 이하)에만 얹습니다: 단독주택 60%, 상가주택 20%, 공공 30% (**가정**), 박공·모임지붕 반반, 경사 28°. 공장·창고 사각형은 낮은 경사(10°) 금속 지붕.
- 색은 건물마다 OSM id로 후보 중 하나를 골라 실행마다 같습니다. 색 후보(붉은 벽돌, 베이지 타일, 흰 도장 등)는 눈으로 고른 값입니다.
- Google 3D 타일·항공사진은 라이선스 때문에 쓰지 않습니다. 다른 게임의 에셋도 쓰지 않습니다.
- **2단계 세부 (#59)** — 모두 표시용 **가정**이며 실측·통계 근거는 아직 없습니다.
  - 1층 상점 유리·간판 띠, 차양, 돌출 간판은 "도로에 면한 벽"에만 붙입니다: 벽 가운데에서 바깥쪽으로 도로 가장자리(차로 수 × 1.5 m)까지 12 m 안에 도로 중심선이 있는 벽. 차양은 칸의 절반쯤(높이 2.2 → 1.85 m, 1.1 m 돌출), 돌출 간판은 3층 이상 상가주택 벽의 55%에 하나(3.3 m부터 최대 6 m 높이).
  - 아파트 측벽 동 번호: 무게중심이 80 m 안인 아파트를 한 단지로 묶고, 20 m 띠 기준 북→남, 같은 띠에서는 서→동 순으로 101, 102, … 를 붙입니다. 실제 동 번호가 아닙니다. 건물 데이터에 실제 동 이름이 생기면 그것을 씁니다.
  - 아파트 옥상에는 긴 축을 따라 승강기·계단실 옥탑 1~3개(길이 40 m 미만 1개, 75 m 미만 2개, 그 이상 3개, 6 m × 최대 6 m × 3.6 m).
  - 저층 평지붕: 옥탑방·계단실(단독 30%, 상가주택 55%, 상업 70%, 공공 50%), 물탱크(45%, 파랑·노랑·흰색, 옥탑이 있으면 그 위), 실외기(상업 2~5개, 상가주택 0~2개). 바닥 다각형 안(0.5 m 여유)에 들어갈 때만 놓습니다.
  - 건물 데이터: 건축물대장 자료에 좌표가 없어 건물별 실제 층수·용도는 아직 붙지 않았습니다 (2단계 코어 작업). 붙으면 표시 분류를 그 값으로 바꿉니다.
  - 비용: 건물 메시 꼭짓점 약 23만 → 55만 개, 헤드리스 로드 시간 약 0.4초 늘어남 (평택역 맵, 이 PC).

## 해·하늘 조명 (클라이언트 표시)

<a id="sky-light"></a>
**태양 위치 — 계산식, 시각·계절 연결은 가정** — `client/scripts/sky_light.gd` (#62). 조사일: 2026-11-28.
- 해 위치는 맵 `manifest.json`의 원점 위도·경도와 날짜·시각으로 계산합니다: 적위와 균시차는 널리 쓰이는 Spencer(1971) 푸리에 급수 근사이고, 한국 표준시(UTC+9, 기준 경선 135°E)에 경도 차 × 4분과 균시차를 더해 진태양시를 구합니다. 굴절은 넣지 않았습니다. 계산값(평택역, 위도 36.99°): 남중 고도 1월 15일 31.7°(12:40), 7월 15일 74.7°(12:37). 하지 무렵 남중 고도는 90° − 37° + 23.4° ≈ 76°, 동지 무렵은 ≈ 29.6°라 계절 차이가 맞습니다. 7월 15일 기하학적 해짐은 19:48입니다 (실제 일몰 시각은 굴절 때문에 몇 분 늦음). 근거 페이지를 따로 확인하지 않은 계산식이라 **표시용**으로만 씁니다.
- 하늘·노을 색, 밤 노출, 달빛(약한 푸른 방향광, 그림자 없음)은 눈으로 맞춘 값입니다. 창 불빛은 해 고도 6° 아래부터 켜지고, 밤에는 창 칸의 약 38%가 켜집니다 (**가정**, 통계 없음).
- 계절 대표일: 봄 4/15, 여름 7/15, 가을 10/15, 겨울 1/15. 지형 색은 계절마다 조금 바꿉니다(가을은 누렇게). 나무·눈은 아직 없습니다.
- 게임 안에서는 조명이 시뮬레이션 시각(`time_s`)을 따릅니다. 코어의 계절 진행 API(ADR 0007)에는 아직 "지금 계절" 값이 없어서, 계절은 K 키와 `--season`으로 고릅니다. 코어에 계절 값이 생기면 그것을 따르게 바꿉니다.
