extends Node3D
## 가로수와 가로등: 도로 등급별로 도로를 따라 절차 배치하고 멀티메시로 그린다.
##
## 근거와 가정: docs/design/research-log.md#streetscape
##   - 가로수 간격 6~8 m (Jeong & Lim 2019, 전주시 조례 권고), 수종 비율은 서울 2004 통계를 빌려 씀
##   - 가로등 12 m 등주·55 m 간격·마주보기는 KS A 3701 고속도로 예 (Shin 외 2016). 도시 도로 값은 가정
##
##   build(world, height_at)  도로가 바뀌면 다시 부른다
##   set_season(name)         spring·summer·autumn·winter: 나뭇잎 색과 크기
##   set_night(n)             0 낮 ~ 1 밤: 가로등 불빛

## 도로 등급별 배치 규칙
##   tree: 가로수 간격 m (0 = 없음), lamp: 가로등 간격 m, both: 가로등 양쪽 마주보기, pole: 등주 높이 m
const RULES := {
	"primary": {"tree": 8.0, "lamp": 40.0, "both": true, "pole": 12.0},
	"secondary": {"tree": 8.0, "lamp": 40.0, "both": true, "pole": 12.0},
	"tertiary": {"tree": 8.0, "lamp": 35.0, "both": false, "pole": 10.0},
	"residential": {"tree": 0.0, "lamp": 30.0, "both": false, "pole": 8.0},
	"unclassified": {"tree": 0.0, "lamp": 30.0, "both": false, "pole": 8.0},
	"living_street": {"tree": 0.0, "lamp": 30.0, "both": false, "pole": 8.0},
}
const LANE_HALF_M := 1.5           # 차로 폭 3 m의 절반 (map_view.LANE_WIDTH_M)
const TREE_SETBACK_M := 2.0        # 차도 가장자리에서 가로수까지 (보도 식재대)
const LAMP_SETBACK_M := 0.7        # 차도 가장자리에서 등주까지
const END_CLEAR_M := 12.0          # 도로 끝(교차로) 근처는 비움
const GRID_M := 30.0

## 수종: 비율은 서울 가로수 2004 통계(은행 42%, 버즘 37%, 느티 7%, 벚 4%)를 다시 나눈 값
## shape: 수관 반지름(가로), 높이(세로), 지하고 / 계절 색 (sRGB)
const SPECIES := [
	{"name": "은행나무", "share": 0.46, "r": 2.2, "h": 6.5, "trunk": 2.5,
		"spring": Color(0.55, 0.72, 0.35), "summer": Color(0.30, 0.50, 0.22), "autumn": Color(0.95, 0.80, 0.18), "winter": Color(0.42, 0.36, 0.30)},
	{"name": "버즘나무", "share": 0.40, "r": 3.6, "h": 6.0, "trunk": 3.0,
		"spring": Color(0.50, 0.68, 0.32), "summer": Color(0.27, 0.45, 0.20), "autumn": Color(0.62, 0.48, 0.22), "winter": Color(0.45, 0.40, 0.34)},
	{"name": "느티나무", "share": 0.08, "r": 3.8, "h": 5.0, "trunk": 2.8,
		"spring": Color(0.52, 0.70, 0.30), "summer": Color(0.28, 0.48, 0.22), "autumn": Color(0.78, 0.42, 0.16), "winter": Color(0.40, 0.35, 0.30)},
	{"name": "벚나무", "share": 0.06, "r": 2.8, "h": 3.6, "trunk": 2.2,
		"spring": Color(0.98, 0.80, 0.86), "summer": Color(0.30, 0.48, 0.24), "autumn": Color(0.80, 0.30, 0.18), "winter": Color(0.40, 0.33, 0.30)},
]

var trees: Array = []        # [위치 Vector3, 수종 번호, 크기 배율]
var lamps: Array = []        # [밑동 Vector3, 도로 쪽 방향 Vector3, 등주 높이]
var season := "summer"
var night := 0.0

var trunk_mm: MultiMeshInstance3D
var canopy_mm: MultiMeshInstance3D
var pole_mm: MultiMeshInstance3D
var head_mm: MultiMeshInstance3D
var glow_mm: MultiMeshInstance3D
var head_mat: StandardMaterial3D
var glow_mat: StandardMaterial3D

var road_grid := {}
var building_grid := {}


func _init() -> void:
	name = "Streetscape"


func build(world: RefCounted, height_at: Callable) -> void:
	trees.clear()
	lamps.clear()
	_index(world)
	for e in world.road_count():
		if world.road_removed(e):
			continue
		var hw: String = world.road_highway(e).trim_suffix("_link")
		if not RULES.has(hw):
			continue
		var rule: Dictionary = RULES[hw]
		var pts: PackedVector2Array = world.road_points(e)
		var half: float = maxf(world.road_lanes(e), 1) * LANE_HALF_M
		var sp := _species_for(e)
		if rule["tree"] > 0.0:
			for side in [1.0, -1.0]:
				for s in _along(pts, rule["tree"], half + TREE_SETBACK_M, side, rule["tree"] * 0.5):
					var p: Vector2 = s[0]
					if _clear(p, e, 1.2):
						var scale := 0.85 + 0.3 * _hash(p)
						trees.append([Vector3(p.x, height_at.call(p), -p.y), sp, scale])
		var sides := [1.0, -1.0] if rule["both"] else [1.0]
		for side in sides:
			for s in _along(pts, rule["lamp"], half + LAMP_SETBACK_M, side, rule["lamp"] * 0.5):
				var p: Vector2 = s[0]
				var inward: Vector2 = s[1] * -side
				if _clear(p, e, 0.6):
					lamps.append([Vector3(p.x, height_at.call(p), -p.y), Vector3(inward.x, 0, -inward.y), rule["pole"]])
	_make_meshes()
	set_season(season)
	set_night(night)


## 도로를 따라 간격 spacing으로, 중심선에서 offset 떨어진 점들: [[점, 오른쪽 법선], …]
## 처음은 phase에서 시작하고, 양 끝 END_CLEAR_M 안은 건너뛴다.
func _along(pts: PackedVector2Array, spacing: float, offset: float, side: float, phase: float) -> Array:
	var out := []
	var total := 0.0
	for i in pts.size() - 1:
		total += pts[i].distance_to(pts[i + 1])
	if total < 2.0 * END_CLEAR_M + 1.0:
		return out
	var t := phase
	var walked := 0.0
	for i in pts.size() - 1:
		var a := pts[i]
		var b := pts[i + 1]
		var seg := a.distance_to(b)
		if seg < 0.01:
			continue
		var d := (b - a) / seg
		var n := Vector2(d.y, -d.x)
		while t <= walked + seg:
			if t >= END_CLEAR_M and t <= total - END_CLEAR_M:
				out.append([a + d * (t - walked) + n * offset * side, n])
			t += spacing
		walked += seg
	return out


## 다른 도로 차도 위나 건물 안이 아니면 참. margin은 다른 도로 가장자리에서 띄울 거리.
func _clear(p: Vector2, own_edge: int, margin: float) -> bool:
	var c := Vector2i(floori(p.x / GRID_M), floori(p.y / GRID_M))
	for gx in range(c.x - 1, c.x + 2):
		for gy in range(c.y - 1, c.y + 2):
			var key := Vector2i(gx, gy)
			for sg in road_grid.get(key, []):
				if sg[3] == own_edge:
					continue
				var q := Geometry2D.get_closest_point_to_segment(p, sg[0], sg[1])
				if q.distance_to(p) < sg[2] + margin:
					return false
	for fp in building_grid.get(c, []):
		if Geometry2D.is_point_in_polygon(p, fp):
			return false
	return true


func _index(world: RefCounted) -> void:
	road_grid.clear()
	building_grid.clear()
	for e in world.road_count():
		if world.road_removed(e):
			continue
		var pts: PackedVector2Array = world.road_points(e)
		var half: float = maxf(world.road_lanes(e), 1) * LANE_HALF_M
		for i in pts.size() - 1:
			for key in _cells(pts[i], pts[i + 1]):
				if not road_grid.has(key):
					road_grid[key] = []
				road_grid[key].append([pts[i], pts[i + 1], half, e])
	for i in world.building_count():
		var fp: PackedVector2Array = world.building_footprint(i)
		if fp.size() < 3:
			continue
		var lo := Vector2(INF, INF)
		var hi := Vector2(-INF, -INF)
		for p in fp:
			lo = lo.min(p)
			hi = hi.max(p)
		for key in _cells(lo, hi):
			if not building_grid.has(key):
				building_grid[key] = []
			building_grid[key].append(fp)


static func _cells(a: Vector2, b: Vector2) -> Array:
	var out := []
	for gx in range(floori(minf(a.x, b.x) / GRID_M), floori(maxf(a.x, b.x) / GRID_M) + 1):
		for gy in range(floori(minf(a.y, b.y) / GRID_M), floori(maxf(a.y, b.y) / GRID_M) + 1):
			out.append(Vector2i(gx, gy))
	return out


## 같은 도로(노선)에는 같은 수종. 도로 번호로 고정해 고른다.
func _species_for(edge: int) -> int:
	var t := fposmod(sin(edge * 12.9898 + 4.1) * 43758.5453, 1.0)
	var acc := 0.0
	for k in SPECIES.size():
		acc += SPECIES[k]["share"]
		if t < acc:
			return k
	return 0


static func _hash(p: Vector2) -> float:
	return fposmod(sin(p.x * 12.9898 + p.y * 78.233) * 43758.5453, 1.0)


# ── 그리기 ──────────────────────────────────────────

func _make_meshes() -> void:
	for n in [trunk_mm, canopy_mm, pole_mm, head_mm, glow_mm]:
		if n:
			remove_child(n)
			n.queue_free()

	var bark := _mat(Color(0.33, 0.27, 0.22))
	var trunk := CylinderMesh.new()
	trunk.top_radius = 0.15
	trunk.bottom_radius = 0.22
	trunk.height = 1.0
	trunk.radial_segments = 6
	trunk.rings = 1
	trunk_mm = _mm("TreeTrunks", trunk, bark, trees.size(), false)

	var leaf := StandardMaterial3D.new()
	leaf.vertex_color_use_as_albedo = true
	leaf.vertex_color_is_srgb = true
	leaf.roughness = 0.95
	var canopy := SphereMesh.new()
	canopy.radius = 0.5
	canopy.height = 1.0
	canopy.radial_segments = 8
	canopy.rings = 5
	canopy_mm = _mm("TreeCanopies", canopy, leaf, trees.size(), true)

	var metal := _mat(Color(0.42, 0.44, 0.46))
	metal.metallic = 0.4
	metal.roughness = 0.5
	var pole := CylinderMesh.new()
	pole.top_radius = 0.08
	pole.bottom_radius = 0.12
	pole.height = 1.0
	pole.radial_segments = 6
	pole.rings = 1
	pole_mm = _mm("LampPoles", pole, metal, lamps.size(), false)

	# 등기구: 팔(암) 끝의 납작한 상자, 밤에 빛남
	head_mat = _mat(Color(0.30, 0.31, 0.33))
	head_mat.emission_enabled = true
	head_mat.emission = Color(1.0, 0.82, 0.55)
	var head := BoxMesh.new()
	head.size = Vector3(1.0, 0.15, 0.45)
	head_mm = _mm("LampHeads", head, head_mat, lamps.size(), false)

	# 노면 불빛 웅덩이: 밤에만 더해지는 원판
	glow_mat = StandardMaterial3D.new()
	glow_mat.shading_mode = BaseMaterial3D.SHADING_MODE_UNSHADED
	glow_mat.transparency = BaseMaterial3D.TRANSPARENCY_ALPHA
	glow_mat.blend_mode = BaseMaterial3D.BLEND_MODE_ADD
	glow_mat.albedo_texture = _radial_texture()
	glow_mat.no_depth_test = false
	var glow := PlaneMesh.new()
	glow.size = Vector2(1, 1)
	glow_mm = _mm("LampGlow", glow, glow_mat, lamps.size(), false)
	glow_mm.cast_shadow = GeometryInstance3D.SHADOW_CASTING_SETTING_OFF

	for i in trees.size():
		var p: Vector3 = trees[i][0]
		var sp: Dictionary = SPECIES[trees[i][1]]
		var k: float = trees[i][2]
		var th: float = sp["trunk"] * k
		trunk_mm.multimesh.set_instance_transform(i, Transform3D(Basis().scaled(Vector3(1, th + sp["h"] * k * 0.4, 1)), p + Vector3(0, (th + sp["h"] * k * 0.4) / 2, 0)))
	for i in lamps.size():
		var p: Vector3 = lamps[i][0]
		var inward: Vector3 = lamps[i][1]
		var hgt: float = lamps[i][2]
		pole_mm.multimesh.set_instance_transform(i, Transform3D(Basis().scaled(Vector3(1, hgt, 1)), p + Vector3(0, hgt / 2, 0)))
		var arm := 1.8
		var hp := p + inward * arm + Vector3(0, hgt, 0)
		var basis := Basis.looking_at(inward, Vector3.UP)
		head_mm.multimesh.set_instance_transform(i, Transform3D(basis.scaled(Vector3(1, 1, 1)), hp))
		# 불빛 웅덩이: 등주 높이에 비례하는 지름, 노면 위 0.7 m (도로 띠 0.6 m 위)
		var r := hgt * 1.6
		glow_mm.multimesh.set_instance_transform(i, Transform3D(Basis().scaled(Vector3(r, 1, r)), p + inward * (arm + 1.0) + Vector3(0, 0.75, 0)))


func _mm(node_name: String, mesh: Mesh, mat: Material, count: int, colors: bool) -> MultiMeshInstance3D:
	var mm := MultiMesh.new()
	mm.transform_format = MultiMesh.TRANSFORM_3D
	mm.use_colors = colors
	mm.mesh = mesh
	mm.instance_count = count
	var mi := MultiMeshInstance3D.new()
	mi.name = node_name
	mi.multimesh = mm
	mi.material_override = mat
	add_child(mi)
	return mi


static func _mat(c: Color) -> StandardMaterial3D:
	var m := StandardMaterial3D.new()
	m.albedo_color = c
	m.roughness = 0.9
	return m


static func _radial_texture() -> GradientTexture2D:
	var g := Gradient.new()
	g.set_color(0, Color(1, 1, 1, 1))
	g.set_color(1, Color(1, 1, 1, 0))
	var t := GradientTexture2D.new()
	t.gradient = g
	t.fill = GradientTexture2D.FILL_RADIAL
	t.fill_from = Vector2(0.5, 0.5)
	t.fill_to = Vector2(1.0, 0.5)
	t.width = 64
	t.height = 64
	return t


func set_season(s: String) -> void:
	season = s
	if canopy_mm == null:
		return
	var bare := season == "winter"
	for i in trees.size():
		var p: Vector3 = trees[i][0]
		var sp: Dictionary = SPECIES[trees[i][1]]
		var k: float = trees[i][2]
		var r: float = sp["r"] * k * (0.55 if bare else 1.0)
		var h: float = sp["h"] * k * (0.7 if bare else 1.0)
		var base: float = sp["trunk"] * k
		canopy_mm.multimesh.set_instance_transform(i, Transform3D(Basis().scaled(Vector3(r * 2, h, r * 2)), p + Vector3(0, base + h / 2, 0)))
		var c: Color = sp.get(season, sp["summer"])
		var v := 0.9 + 0.2 * _hash(Vector2(p.x, p.z))
		canopy_mm.multimesh.set_instance_color(i, Color(c.r * v, c.g * v, c.b * v))


func set_night(n: float) -> void:
	night = n
	if head_mat:
		head_mat.emission_energy_multiplier = 4.0 * night
		glow_mat.albedo_color = Color(1.0, 0.78, 0.5, 0.55 * night)
		glow_mm.visible = night > 0.01
