extends RefCounted
## 건물 메시: 바닥 다각형 → 벽·지붕 (용도별 색, 창문 셰이더, 평지붕 난간, 박공·모임지붕).
##
## 표시 분류와 색·지붕 규칙의 근거와 가정: docs/design/research-log.md#building-look
## 셰이더 정점 데이터 형식: shaders/building.gdshader

enum Style { ROOF, APARTMENT, SHOPHOUSE, HOUSE, COMMERCIAL, CIVIC, INDUSTRIAL, STATION, BLANK }

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

var st := SurfaceTool.new()
var stats := {}


## world: DongneWorld, height_at: Callable(Vector2) -> float (지형 높이)
func build(world: RefCounted, height_at: Callable) -> MeshInstance3D:
	st.begin(Mesh.PRIMITIVE_TRIANGLES)
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
		_building(fp, base, h, kind, seed)
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


func _building(fp: PackedVector2Array, base: float, h: float, kind: String, seed: float) -> void:
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
	_walls(fp, base, h, h + parapet, style, wall)

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


## 벽: 바닥 base에서 높이 wall_h까지 창문 벽, total_h까지 난간.
func _walls(fp: PackedVector2Array, base: float, wall_h: float, total_h: float, style: int, color: Color) -> void:
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
		var n := Vector3(d.y, 0, d.x) / seg     # 바깥쪽 (반시계 다각형)
		var q := [_v(a, base), _v(b, base), _v(b, base + total_h), _v(a, base + total_h)]
		var uv := [Vector2(0, 0), Vector2(bays, 0), Vector2(bays, total_h), Vector2(0, total_h)]
		var uv2 := Vector2(s, wall_h)
		_quad(q, uv, uv2, n, color)


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
		st.add_vertex(pts[k])


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
