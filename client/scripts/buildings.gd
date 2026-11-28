extends RefCounted
## 건물 메시: 바닥 다각형 → 벽·지붕 (용도별 색, 창문 셰이더, 평지붕 난간, 박공·모임지붕)
## + 유형별 세부: 아파트 옥탑·동 번호, 도로 쪽 상점 차양·돌출 간판, 저층 옥상 물탱크·옥탑방·실외기.
##
## 표시 분류와 색·지붕 규칙의 근거와 가정: docs/design/research-log.md#building-look
## 셰이더 정점 데이터 형식: shaders/building.gdshader

enum Style { ROOF, APARTMENT, SHOPHOUSE, HOUSE, COMMERCIAL, CIVIC, INDUSTRIAL, STATION, BLANK, SIGN }
## 셰이더 UV2.x에 더하는 값: 도로에 면한 벽 (1층 상점·간판을 그림)
const STREET_FLAG := 16

const SHADER := preload("res://shaders/building.gdshader")

## 분류별 창 간격 (m). 벽 한 변에 들어갈 칸 수 = round(변 길이 / 간격)
const BAY_M := {
	Style.APARTMENT: 3.3, Style.SHOPHOUSE: 3.0, Style.HOUSE: 3.5, Style.COMMERCIAL: 2.6,
	Style.CIVIC: 3.6, Style.INDUSTRIAL: 6.0, Style.STATION: 3.0,
}

## 벽 색 후보 (sRGB). 건물마다 OSM id로 하나를 고른다.
const WALL_COLORS := {
	Style.APARTMENT: [Color(0.90, 0.88, 0.82), Color(0.86, 0.85, 0.83), Color(0.92, 0.88, 0.78), Color(0.84, 0.86, 0.88)],
	Style.SHOPHOUSE: [Color(0.62, 0.36, 0.28), Color(0.55, 0.32, 0.26), Color(0.80, 0.72, 0.60), Color(0.70, 0.68, 0.64), Color(0.86, 0.84, 0.80), Color(0.74, 0.62, 0.50)],
	Style.HOUSE: [Color(0.66, 0.40, 0.30), Color(0.88, 0.85, 0.78), Color(0.80, 0.74, 0.62), Color(0.72, 0.70, 0.66)],
	Style.COMMERCIAL: [Color(0.62, 0.68, 0.72), Color(0.72, 0.72, 0.70), Color(0.80, 0.78, 0.74)],
	Style.CIVIC: [Color(0.86, 0.80, 0.66), Color(0.64, 0.40, 0.32), Color(0.84, 0.84, 0.80)],
	Style.INDUSTRIAL: [Color(0.70, 0.74, 0.78), Color(0.76, 0.76, 0.74), Color(0.62, 0.70, 0.78)],
	Style.STATION: [Color(0.82, 0.84, 0.86)],
	Style.BLANK: [Color(0.66, 0.66, 0.64)],
}
## 평지붕 색: 저층은 녹색 우레탄 방수 도장이 흔하고, 아파트·큰 건물은 회색 콘크리트,
## 일부는 흰색 쿨루프 (research-log.md#building-look)
const ROOF_COOL := Color(0.88, 0.89, 0.88)
const ROOF_GREEN := Color(0.36, 0.56, 0.44)
const ROOF_CONCRETE := [Color(0.64, 0.64, 0.62), Color(0.70, 0.69, 0.66), Color(0.58, 0.59, 0.60)]
## 경사지붕 색: 기와·슬레이트·금속 (회색, 적갈색, 청색, 녹색)
const ROOF_PITCHED := [Color(0.30, 0.32, 0.35), Color(0.52, 0.26, 0.20), Color(0.26, 0.38, 0.54), Color(0.30, 0.44, 0.36), Color(0.44, 0.42, 0.40)]
const ROOF_METAL := [Color(0.38, 0.50, 0.62), Color(0.58, 0.60, 0.62)]

const PARAPET_M := 0.9           # 평지붕 난간 높이
const HOUSE_MAX_AREA := 100.0    # building=yes 중 이보다 작으면 단독주택으로 봄 (m²)
const LARGE_MIN_AREA := 700.0    # building=yes 중 이보다 크면 상업 건물로 봄 (m²)
const PITCHED_MAX_AREA := 260.0  # 경사지붕을 얹는 사각형 건물의 최대 바닥 면적 (m²)
const STREET_M := 12.0           # 도로 가장자리에서 이 거리 안의 벽을 "도로에 면한 벽"으로 봄
const COMPLEX_M := 80.0          # 아파트 무게중심이 이 거리 안이면 같은 단지로 묶어 동 번호를 매김
const ROAD_GRID_M := 30.0

## 돌출 간판·차양·물탱크 색 (sRGB)
const SIGN_COLORS := [Color(0.80, 0.16, 0.14), Color(0.95, 0.80, 0.20), Color(0.16, 0.36, 0.75), Color(0.92, 0.92, 0.90), Color(0.15, 0.55, 0.32), Color(0.95, 0.50, 0.15)]
const AWNING_COLORS := [Color(0.75, 0.20, 0.18), Color(0.20, 0.45, 0.30), Color(0.22, 0.35, 0.62), Color(0.88, 0.62, 0.18), Color(0.85, 0.85, 0.82)]
const TANK_COLORS := [Color(0.25, 0.45, 0.75), Color(0.92, 0.76, 0.25), Color(0.88, 0.88, 0.85)]
const AC_COLOR := Color(0.80, 0.80, 0.78)

var st := SurfaceTool.new()
var stats := {}
var road_grid := {}        # Vector2i → [[a, b, 반폭], …]
var custom := Color(0, 0, 0, 0)   # 셰이더 CUSTOM0: (동 번호, 벽 길이 m, 0, 0)


## world: DongneWorld, height_at: Callable(Vector2) -> float (지형 높이)
func build(world: RefCounted, height_at: Callable) -> MeshInstance3D:
	st.begin(Mesh.PRIMITIVE_TRIANGLES)
	st.set_custom_format(0, SurfaceTool.CUSTOM_RGBA_FLOAT)
	_build_road_grid(world)
	var numbers := _apartment_numbers(world)
	for i in world.building_count():
		var fp: PackedVector2Array = world.building_footprint(i)
		if fp.size() < 3:
			continue
		if _signed_area(fp) < 0:
			fp.reverse()
		var h: float = world.building_height(i)
		var kind: String = world.building_kind(i)
		var seed := _seed(world.building_osm_id(i))
		var base := INF
		for p in fp:
			base = minf(base, height_at.call(p))
		_building(fp, base, h, kind, seed, numbers.get(i, 0))
	var mi := MeshInstance3D.new()
	mi.name = "Buildings"
	mi.mesh = st.commit()
	var mat := ShaderMaterial.new()
	mat.shader = SHADER
	mi.material_override = mat
	return mi


## OSM 값과 바닥 면적으로 표시 분류를 고른다.
static func classify(kind: String, area: float) -> int:
	match kind:
		"apartments":
			return Style.APARTMENT
		"house", "detached", "semidetached_house", "terrace", "bungalow":
			return Style.HOUSE
		"residential", "dormitory":
			return Style.SHOPHOUSE
		"commercial", "retail", "office", "hotel", "supermarket":
			return Style.COMMERCIAL
		"school", "university", "college", "kindergarten", "public", "civic", "hospital", "church", "government":
			return Style.CIVIC
		"industrial", "warehouse", "factory":
			return Style.INDUSTRIAL
		"train_station", "transportation":
			return Style.STATION
		"roof", "gazebo", "parking", "construction", "shed", "garage", "garages", "kiosk", "carport", "hut":
			return Style.BLANK
	# building=yes 등: 면적으로 나눈다 (가정)
	if area < HOUSE_MAX_AREA:
		return Style.HOUSE
	if area > LARGE_MIN_AREA:
		return Style.COMMERCIAL
	return Style.SHOPHOUSE


func _building(fp: PackedVector2Array, base: float, h: float, kind: String, seed: float, number: int) -> void:
	var area := _signed_area(fp)
	var style := classify(kind, area)
	stats[style] = stats.get(style, 0) + 1
	var wall: Color = _pick(WALL_COLORS[style], seed)
	if kind == "construction":
		wall = Color(0.78, 0.70, 0.42)
	wall.a = seed
	var top := base + h

	# 경사지붕: 볼록한 사각형이고 작은 저층 건물 (주택은 대부분, 상가주택은 일부, 공장은 낮은 경사)
	var pitched := false
	if fp.size() == 4 and _convex(fp) and area <= PITCHED_MAX_AREA and h <= 12.0:
		match style:
			Style.HOUSE:
				pitched = _rand(seed, 1) < 0.6
			Style.SHOPHOUSE:
				pitched = _rand(seed, 1) < 0.2
			Style.CIVIC:
				pitched = _rand(seed, 1) < 0.3
	if style == Style.INDUSTRIAL and fp.size() == 4 and _convex(fp):
		pitched = true

	var parapet := 0.0 if pitched or style == Style.BLANK else PARAPET_M
	if style == Style.APARTMENT:
		parapet = 1.2
	_walls(fp, base, h, h + parapet, style, wall, seed, number)

	if pitched:
		var roof: Color
		if style == Style.INDUSTRIAL:
			roof = _pick(ROOF_METAL, _rand(seed, 2))
		else:
			roof = _pick(ROOF_PITCHED, _rand(seed, 2))
		roof.a = seed
		var pitch_deg := 10.0 if style == Style.INDUSTRIAL else 28.0
		_pitched_roof(fp, top, pitch_deg, _rand(seed, 3) < 0.5, roof, wall)
	else:
		var roof: Color
		var r := _rand(seed, 2)
		if style in [Style.HOUSE, Style.SHOPHOUSE] and r < 0.6:
			roof = ROOF_GREEN.lerp(Color(0.45, 0.6, 0.5), _rand(seed, 4) * 0.5)
		elif r > 0.92:
			roof = ROOF_COOL
		else:
			roof = _pick(ROOF_CONCRETE, _rand(seed, 2))
		roof.a = seed
		_flat_roof(fp, top, roof)
		if parapet > 0.0:
			_parapet_inside(fp, base, top, top + parapet, roof.darkened(0.08))
		if style == Style.APARTMENT:
			_apartment_cores(fp, top, wall, seed)
		elif style in [Style.HOUSE, Style.SHOPHOUSE, Style.COMMERCIAL, Style.CIVIC]:
			_rooftop_clutter(fp, top, style, wall, seed)


## 벽: 바닥 base에서 높이 wall_h까지 창문 벽, total_h까지 난간.
func _walls(fp: PackedVector2Array, base: float, wall_h: float, total_h: float, style: int, color: Color, seed: float, number: int) -> void:
	var max_len := 0.0
	for j in fp.size():
		max_len = maxf(max_len, fp[j].distance_to(fp[(j + 1) % fp.size()]))
	for j in fp.size():
		var a := fp[j]
		var b := fp[(j + 1) % fp.size()]
		var seg := a.distance_to(b)
		if seg < 0.01:
			continue
		var s := style
		var bays := 0.0
		if BAY_M.has(style):
			bays = roundf(seg / BAY_M[style])
			if bays < 1.0 and seg >= BAY_M[style] * 0.55:
				bays = 1.0
		# 아파트 짧은 옆벽(측벽)은 창이 없다
		if style == Style.APARTMENT and seg < max_len * 0.45:
			bays = 0.0
		if bays < 1.0:
			s = Style.BLANK
		var d := b - a
		var n2 := Vector2(d.y, -d.x) / seg       # 바깥쪽 (반시계 다각형)
		var n := Vector3(n2.x, 0, -n2.y)
		var street := false
		if s in [Style.SHOPHOUSE, Style.COMMERCIAL]:
			street = _faces_road((a + b) / 2, n2)
		var u1 := bays
		# 아파트 측벽 동 번호: 셰이더가 벽 길이(m) 좌표로 숫자를 그린다
		if style == Style.APARTMENT and s == Style.BLANK and number > 0 and seg >= 8.0 and wall_h >= 15.0:
			custom = Color(number, seg, 0, 0)
			u1 = seg
		var q := [_v(a, base), _v(b, base), _v(b, base + total_h), _v(a, base + total_h)]
		var uv := [Vector2(0, 0), Vector2(u1, 0), Vector2(u1, total_h), Vector2(0, total_h)]
		var uv2 := Vector2(s + (STREET_FLAG if street else 0), wall_h)
		_quad(q, uv, uv2, n, color)
		custom = Color(0, 0, 0, 0)
		if street:
			_storefront(a, b, n2, base, bays, wall_h, style, fposmod(seed + j * 0.137, 1.0))


## 평지붕 (바닥 다각형 삼각분할).
func _flat_roof(fp: PackedVector2Array, top: float, color: Color) -> void:
	var tri := Geometry2D.triangulate_polygon(fp)
	for k in range(0, tri.size(), 3):
		_tri(_v(fp[tri[k]], top), _v(fp[tri[k + 1]], top), _v(fp[tri[k + 2]], top),
			Vector3.UP, color, Vector2(Style.ROOF, 0))


## 난간 안쪽 면 (지붕 위에서 보이는 쪽).
func _parapet_inside(fp: PackedVector2Array, base: float, top: float, rim: float, color: Color) -> void:
	for j in fp.size():
		var a := fp[j]
		var b := fp[(j + 1) % fp.size()]
		var seg := a.distance_to(b)
		if seg < 0.01:
			continue
		var d := b - a
		var n := -Vector3(d.y, 0, d.x) / seg
		var q := [_v(a, top), _v(b, top), _v(b, rim), _v(a, rim)]
		var uv := [Vector2(0, top - base), Vector2(0, top - base), Vector2(0, rim - base), Vector2(0, rim - base)]
		_quad(q, uv, Vector2(Style.ROOF, 0), n, color)


## 볼록 사각형 위 박공지붕(hip=false) 또는 모임지붕(hip=true). 용마루는 긴 변과 나란하다.
func _pitched_roof(fp: PackedVector2Array, top: float, pitch_deg: float, hip: bool, roof: Color, wall: Color) -> void:
	var p := fp.duplicate()
	# p0-p1, p2-p3이 짧은 변이 되도록 돌린다
	if p[0].distance_to(p[1]) + p[2].distance_to(p[3]) > p[1].distance_to(p[2]) + p[3].distance_to(p[0]):
		p = PackedVector2Array([p[1], p[2], p[3], p[0]])
	var ma := (p[0] + p[1]) / 2
	var mb := (p[2] + p[3]) / 2
	var width := (p[0].distance_to(p[1]) + p[2].distance_to(p[3])) / 2
	var ridge_len := ma.distance_to(mb)
	var rise := width / 2 * tan(deg_to_rad(pitch_deg))
	var ra := ma
	var rb := mb
	if hip and ridge_len > 0.01:
		var inset := minf(width / 2, ridge_len * 0.45)
		var dir := (mb - ma) / ridge_len
		ra = ma + dir * inset
		rb = mb - dir * inset
	var ridge := top + rise
	var A := _v(ra, ridge)
	var B := _v(rb, ridge)
	var c := [_v(p[0], top), _v(p[1], top), _v(p[2], top), _v(p[3], top)]
	var rs := Vector2(Style.ROOF, 0)
	# 긴 두 면
	_roof_face([c[1], c[2], B, A], roof, rs)
	_roof_face([c[3], c[0], A, B], roof, rs)
	# 양 끝: 모임지붕이면 지붕면, 박공이면 벽(박공벽)
	if hip:
		_roof_face([c[0], c[1], A], roof, rs)
		_roof_face([c[2], c[3], B], roof, rs)
	else:
		var gs := Vector2(Style.BLANK, 1000.0)
		_roof_face([c[0], c[1], A], wall, gs)
		_roof_face([c[2], c[3], B], wall, gs)


## 위·바깥을 보는 면 (삼각형 또는 사각형). 법선은 도형에서 구한다.
func _roof_face(pts: Array, color: Color, uv2: Vector2) -> void:
	var n: Vector3 = (pts[1] - pts[0]).cross(pts[2] - pts[0]).normalized()
	# 위쪽으로, 수직면(박공벽)은 바깥쪽(용마루 반대편)으로
	if n.y < -0.01:
		n = -n
	elif absf(n.y) <= 0.01:
		var mid: Vector3 = (pts[0] + pts[1]) / 2
		if n.dot(mid - pts[2]) < 0:
			n = -n
	_tri(pts[0], pts[1], pts[2], n, color, uv2)
	if pts.size() == 4:
		_tri(pts[0], pts[2], pts[3], n, color, uv2)


func _quad(q: Array, uv: Array, uv2: Vector2, n: Vector3, color: Color) -> void:
	for k in [[0, 1, 2], [0, 2, 3]]:
		_tri(q[k[0]], q[k[1]], q[k[2]], n, color, uv2, [uv[k[0]], uv[k[1]], uv[k[2]]])


## 삼각형 하나. 법선 n 쪽에서 앞면으로 보이게 감는 방향을 맞춘다 (Godot는 시계 방향이 앞면).
func _tri(a: Vector3, b: Vector3, c: Vector3, n: Vector3, color: Color, uv2: Vector2, uvs: Array = []) -> void:
	var ub = uvs[1] if uvs.size() == 3 else null
	var uc = uvs[2] if uvs.size() == 3 else null
	if (c - a).cross(b - a).dot(n) < 0:
		var t := b
		b = c
		c = t
		var tu = ub
		ub = uc
		uc = tu
	var pts := [a, b, c]
	var us := [uvs[0] if uvs.size() == 3 else null, ub, uc]
	for k in 3:
		st.set_normal(n)
		st.set_color(color)
		st.set_uv(us[k] if us[k] != null else Vector2(0, 0))
		st.set_uv2(uv2)
		st.set_custom(0, custom)
		st.add_vertex(pts[k])


# ── 도로에 면한 벽 ──────────────────────────────────────

func _build_road_grid(world: RefCounted) -> void:
	road_grid.clear()
	for e in world.road_count():
		if world.road_removed(e):
			continue
		var pts: PackedVector2Array = world.road_points(e)
		var half: float = maxf(world.road_lanes(e), 1) * 1.5
		for i in pts.size() - 1:
			var a := pts[i]
			var b := pts[i + 1]
			var lo := Vector2i(floori(minf(a.x, b.x) / ROAD_GRID_M), floori(minf(a.y, b.y) / ROAD_GRID_M))
			var hi := Vector2i(floori(maxf(a.x, b.x) / ROAD_GRID_M), floori(maxf(a.y, b.y) / ROAD_GRID_M))
			for gx in range(lo.x, hi.x + 1):
				for gy in range(lo.y, hi.y + 1):
					var key := Vector2i(gx, gy)
					if not road_grid.has(key):
						road_grid[key] = []
					road_grid[key].append([a, b, half])


## 벽 가운데 m에서 바깥쪽 n2 방향으로, 도로 가장자리까지 STREET_M 안에 도로가 있으면 참.
func _faces_road(m: Vector2, n2: Vector2) -> bool:
	var c := Vector2i(floori(m.x / ROAD_GRID_M), floori(m.y / ROAD_GRID_M))
	for gx in range(c.x - 1, c.x + 2):
		for gy in range(c.y - 1, c.y + 2):
			for sg in road_grid.get(Vector2i(gx, gy), []):
				var q := Geometry2D.get_closest_point_to_segment(m, sg[0], sg[1])
				var off := q - m
				var dist := off.length()
				if dist - sg[2] < STREET_M and off.dot(n2) > 0.6 * dist:
					return true
	return false


## 도로에 면한 상가 1층: 칸마다 차양(절반쯤), 3층 이상 상가주택은 돌출 간판.
func _storefront(a: Vector2, b: Vector2, n2: Vector2, base: float, bays: float, wall_h: float, style: int, seed: float) -> void:
	var d := b - a
	var nb := int(bays)
	var rs := Vector2(Style.ROOF, 0)
	var out := n2 * 1.1
	var top := base + 2.2
	var low := base + 1.85
	for k in nb:
		if _rand(seed, 10 + k) >= 0.5:
			continue
		var pa := a + d * ((k + 0.08) / nb)
		var pb := a + d * ((k + 0.92) / nb)
		var col: Color = _pick(AWNING_COLORS, _rand(seed, 30 + k))
		col.a = 0.0
		_roof_face([_v(pa, top), _v(pb, top), _v(pb + out, low), _v(pa + out, low)], col, rs)
		_quad([_v(pa + out, low - 0.3), _v(pb + out, low - 0.3), _v(pb + out, low), _v(pa + out, low)],
			[Vector2.ZERO, Vector2.ZERO, Vector2.ZERO, Vector2.ZERO], rs, Vector3(n2.x, 0, -n2.y), col.darkened(0.15))
	if style == Style.SHOPHOUSE and wall_h >= 8.9 and d.length() >= 6.0 and _rand(seed, 50) < 0.55:
		var t := 0.12 if _rand(seed, 51) < 0.5 else 0.88
		var p := a + d * t + n2 * 0.55
		var sc: Color = _pick(SIGN_COLORS, _rand(seed, 52))
		sc.a = _rand(seed, 53)
		var z1 := base + minf(wall_h - 0.4, 3.3 + 6.0)
		_box(p, n2, 0.9, 0.35, base + 3.3, z1, sc, Vector2(Style.SIGN, 0), true)


# ── 아파트 ──────────────────────────────────────────

## 단지별 동 번호: 무게중심이 COMPLEX_M 안인 아파트를 한 단지로 묶고, 북→남·서→동 순서로 101, 102, …
func _apartment_numbers(world: RefCounted) -> Dictionary:
	var ids: Array = []
	var cen: Array = []
	for i in world.building_count():
		if world.building_kind(i) != "apartments":
			continue
		var fp: PackedVector2Array = world.building_footprint(i)
		var c := Vector2.ZERO
		for p in fp:
			c += p
		ids.append(i)
		cen.append(c / maxf(fp.size(), 1))
	var parent: Array = range(ids.size())
	for x in ids.size():
		for y in range(x + 1, ids.size()):
			if (cen[x] as Vector2).distance_to(cen[y]) < COMPLEX_M:
				parent[_find(parent, x)] = _find(parent, y)
	var groups := {}
	for x in ids.size():
		var r := _find(parent, x)
		if not groups.has(r):
			groups[r] = []
		groups[r].append(x)
	var out := {}
	for r in groups:
		var members: Array = groups[r]
		# 20 m 띠로 북쪽부터, 같은 띠에서는 서쪽부터
		members.sort_custom(func(p, q):
			var bp := -roundi((cen[p] as Vector2).y / 20.0)
			var bq := -roundi((cen[q] as Vector2).y / 20.0)
			return bp < bq if bp != bq else (cen[p] as Vector2).x < (cen[q] as Vector2).x)
		for k in members.size():
			out[ids[members[k]]] = 101 + k
	return out


static func _find(parent: Array, x: int) -> int:
	while parent[x] != x:
		parent[x] = parent[parent[x]]
		x = parent[x]
	return x


## 아파트 옥탑: 긴 축을 따라 승강기·계단실 상자 1~3개.
func _apartment_cores(fp: PackedVector2Array, top: float, wall: Color, seed: float) -> void:
	var box := _obb(fp)
	var c: Vector2 = box[0]
	var dir: Vector2 = box[1]
	var length: float = box[2]
	var width: float = box[3]
	var n := 1 if length < 40.0 else (2 if length < 75.0 else 3)
	var w := minf(width * 0.55, 6.0)
	var col := wall.darkened(0.06)
	col.a = seed
	for k in n:
		var p := c + dir * (((k + 0.5) / n - 0.5) * length)
		if _fits(fp, p, dir, 6.0, w):
			_box(p, dir, 6.0, w, top, top + 3.6, col, Vector2(Style.BLANK, 100.0), true)


# ── 저층 옥상 ──────────────────────────────────────────

## 평지붕 위 옥탑방(계단실), 물탱크, 실외기. 건물마다 OSM id로 고정.
func _rooftop_clutter(fp: PackedVector2Array, top: float, style: int, wall: Color, seed: float) -> void:
	var box := _obb(fp)
	var c: Vector2 = box[0]
	var dir: Vector2 = box[1]
	var length: float = box[2]
	var width: float = box[3]
	var side := Vector2(-dir.y, dir.x)
	var hut_p := Vector2.INF
	var hut_chance := {Style.HOUSE: 0.3, Style.SHOPHOUSE: 0.55, Style.COMMERCIAL: 0.7, Style.CIVIC: 0.5}
	var hl := 4.0 if style == Style.COMMERCIAL else 3.4
	var hw := 3.0 if style == Style.COMMERCIAL else 2.6
	if _rand(seed, 60) < hut_chance[style]:
		for t in 4:
			var p := c + dir * ((_rand(seed, 61 + t) - 0.5) * 0.5 * length) + side * ((_rand(seed, 65 + t) - 0.5) * 0.5 * width)
			if _fits(fp, p, dir, hl, hw):
				var col := wall
				col.a = seed
				_box(p, dir, hl, hw, top, top + 2.6, col, Vector2(Style.BLANK, 100.0), true)
				hut_p = p
				break
	if _rand(seed, 70) < 0.45:
		var tc: Color = _pick(TANK_COLORS, _rand(seed, 71))
		tc.a = 0.0
		var r := 0.7 + 0.4 * _rand(seed, 72)
		if hut_p != Vector2.INF:
			_cylinder(hut_p, r, top + 2.6, top + 2.6 + 1.5, tc)
		else:
			for t in 4:
				var p := c + dir * ((_rand(seed, 73 + t) - 0.5) * 0.6 * length) + side * ((_rand(seed, 77 + t) - 0.5) * 0.6 * width)
				if _fits(fp, p, dir, r * 2.4, r * 2.4):
					_cylinder(p, r, top, top + 1.6, tc)
					break
	var n_ac := 0
	if style == Style.COMMERCIAL:
		n_ac = 2 + int(_rand(seed, 80) * 4)
	elif style == Style.SHOPHOUSE:
		n_ac = int(_rand(seed, 80) * 3)
	for k in n_ac:
		var p := c + dir * ((_rand(seed, 81 + k) - 0.5) * 0.8 * length) + side * ((_rand(seed, 91 + k) - 0.5) * 0.8 * width)
		if _fits(fp, p, dir, 1.4, 1.0) and (hut_p == Vector2.INF or p.distance_to(hut_p) > 3.0):
			var ac := AC_COLOR
			ac.a = 0.0
			_box(p, dir, 0.9, 0.45, top, top + 0.7, ac, Vector2(Style.ROOF, 0), true)


# ── 기본 도형 ──────────────────────────────────────────

## 방향 dir(단위)으로 길이 length, 폭 width인 상자 (바닥 z0, 꼭대기 z1). 옆면은 uv2 분류, 윗면은 지붕.
func _box(c: Vector2, dir: Vector2, length: float, width: float, z0: float, z1: float, color: Color, uv2: Vector2, top_face: bool) -> void:
	var side := Vector2(-dir.y, dir.x)
	var hl := dir * length / 2
	var hw := side * width / 2
	var p := [c - hl - hw, c + hl - hw, c + hl + hw, c - hl + hw]   # 반시계
	var hgt := z1 - z0
	for j in 4:
		var a: Vector2 = p[j]
		var b: Vector2 = p[(j + 1) % 4]
		var d := b - a
		var seg := d.length()
		var n2 := Vector2(d.y, -d.x) / seg
		_quad([_v(a, z0), _v(b, z0), _v(b, z1), _v(a, z1)],
			[Vector2(0, 0), Vector2(seg, 0), Vector2(seg, hgt), Vector2(0, hgt)],
			uv2, Vector3(n2.x, 0, -n2.y), color)
	if top_face:
		var rs := Vector2(Style.ROOF, 0)
		var rc := color if uv2.x == Style.SIGN else color.darkened(0.12)
		_tri(_v(p[0], z1), _v(p[1], z1), _v(p[2], z1), Vector3.UP, rc, rs)
		_tri(_v(p[0], z1), _v(p[2], z1), _v(p[3], z1), Vector3.UP, rc, rs)


func _cylinder(c: Vector2, r: float, z0: float, z1: float, color: Color, segs: int = 10) -> void:
	var rs := Vector2(Style.ROOF, 0)
	var zero4 := [Vector2.ZERO, Vector2.ZERO, Vector2.ZERO, Vector2.ZERO]
	for k in segs:
		var a0 := TAU * k / segs
		var a1 := TAU * (k + 1) / segs
		var pa := c + Vector2(cos(a0), sin(a0)) * r
		var pb := c + Vector2(cos(a1), sin(a1)) * r
		var am := (a0 + a1) / 2
		var n := Vector3(cos(am), 0, -sin(am))
		_quad([_v(pa, z0), _v(pb, z0), _v(pb, z1), _v(pa, z1)], zero4, rs, n, color)
		_tri(_v(c, z1), _v(pa, z1), _v(pb, z1), Vector3.UP, color.lightened(0.08), rs)


## 바닥 다각형의 방향 상자: [중심, 긴 변 방향(단위), 길이, 폭]. 방향은 가장 긴 변.
static func _obb(fp: PackedVector2Array) -> Array:
	var dir := Vector2.RIGHT
	var best := 0.0
	for j in fp.size():
		var d := fp[(j + 1) % fp.size()] - fp[j]
		if d.length() > best:
			best = d.length()
			dir = d / best
	var side := Vector2(-dir.y, dir.x)
	var lo := Vector2(INF, INF)
	var hi := Vector2(-INF, -INF)
	for p in fp:
		var q := Vector2(p.dot(dir), p.dot(side))
		lo = lo.min(q)
		hi = hi.max(q)
	var mid := (lo + hi) / 2
	return [dir * mid.x + side * mid.y, dir, hi.x - lo.x, hi.y - lo.y]


## 상자 네 모서리(0.5 m 여유)가 모두 바닥 다각형 안이면 참.
static func _fits(fp: PackedVector2Array, c: Vector2, dir: Vector2, length: float, width: float) -> bool:
	var side := Vector2(-dir.y, dir.x)
	for sx in [-1, 1]:
		for sy in [-1, 1]:
			var p: Vector2 = c + dir * (sx * (length / 2 + 0.5)) + side * (sy * (width / 2 + 0.5))
			if not Geometry2D.is_point_in_polygon(p, fp):
				return false
	return true


static func _v(p: Vector2, h: float) -> Vector3:
	return Vector3(p.x, h, -p.y)


static func _signed_area(fp: PackedVector2Array) -> float:
	var s := 0.0
	for j in fp.size():
		var a := fp[j]
		var b := fp[(j + 1) % fp.size()]
		s += a.x * b.y - b.x * a.y
	return s / 2


static func _convex(fp: PackedVector2Array) -> bool:
	var sgn := 0.0
	for j in fp.size():
		var a := fp[j]
		var b := fp[(j + 1) % fp.size()]
		var c := fp[(j + 2) % fp.size()]
		var cr := (b - a).cross(c - b)
		if absf(cr) < 1e-6:
			continue
		if sgn == 0.0:
			sgn = signf(cr)
		elif signf(cr) != sgn:
			return false
	return true


## OSM id → 0~1 고정 값 (실행마다, 기계마다 같다)
static func _seed(osm_id: int) -> float:
	return float(posmod(str(osm_id).hash(), 100003)) / 100003.0


static func _rand(seed: float, k: int) -> float:
	return fposmod(sin(seed * 12.9898 + k * 78.233) * 43758.5453, 1.0)


static func _pick(arr: Array, t: float) -> Color:
	return arr[mini(int(t * arr.size()), arr.size() - 1)]
