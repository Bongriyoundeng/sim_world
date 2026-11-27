# tools/

빌드와 원본 데이터 내려받기 스크립트를 둡니다. 원본 OSM·DEM 파일은 `data-raw/`에 받고 커밋하지 않습니다.

| 스크립트 | 내용 |
| --- | --- |
| `build_godot_addon.py` | `core/crates/godot`을 빌드해 `client/addons/dongne_core/bin/`에 복사 (`--debug`로 debug 빌드) |
| `make_demo_gif.py` | 클라이언트 `--demo`가 저장한 프레임을 README용 GIF로 묶기 (Pillow 필요) |
