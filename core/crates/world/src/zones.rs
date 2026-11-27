//! 셀 단위 구역 맵 (terrain과 같은 격자, 행 0 = 북쪽). 통근 수요 생성(#7)의 입력.

pub use dongne_api::Zone;
use dongne_api::{Point, ZoneArea};

#[derive(Debug, Clone, PartialEq)]
pub struct ZoneMap {
    pub rows: u32,
    pub cols: u32,
    pub resolution_m: f64,
    pub width_m: f64,
    pub height_m: f64,
    cells: Vec<Option<Zone>>,
}

impl ZoneMap {
    pub fn new(width_m: f64, height_m: f64, resolution_m: f64) -> Self {
        let rows = (height_m / resolution_m).ceil().max(0.0) as u32;
        let cols = (width_m / resolution_m).ceil().max(0.0) as u32;
        Self {
            rows,
            cols,
            resolution_m,
            width_m,
            height_m,
            cells: vec![None; (rows * cols) as usize],
        }
    }

    pub fn get(&self, row: u32, col: u32) -> Option<Zone> {
        if row < self.rows && col < self.cols {
            self.cells[(row * self.cols + col) as usize]
        } else {
            None
        }
    }

    /// 셀 중심의 로컬 좌표.
    pub fn cell_center(&self, row: u32, col: u32) -> (f64, f64) {
        (
            -self.width_m / 2.0 + (col as f64 + 0.5) * self.resolution_m,
            self.height_m / 2.0 - (row as f64 + 0.5) * self.resolution_m,
        )
    }

    /// 좌표가 들어 있는 셀.
    pub fn cell_at(&self, x: f64, y: f64) -> Option<(u32, u32)> {
        let c = ((x + self.width_m / 2.0) / self.resolution_m).floor();
        let r = ((self.height_m / 2.0 - y) / self.resolution_m).floor();
        (r >= 0.0 && c >= 0.0 && (r as u32) < self.rows && (c as u32) < self.cols)
            .then_some((r as u32, c as u32))
    }

    /// 범위를 셀 목록으로 바꾼다 (행·열 순, 중복 없음). 잘못된 범위면 이유를 돌려준다.
    pub fn resolve(&self, area: &ZoneArea) -> Result<Vec<(u32, u32)>, String> {
        let mut out = Vec::new();
        match area {
            ZoneArea::Cells(cells) => {
                for &[r, c] in cells {
                    if r >= self.rows || c >= self.cols {
                        return Err(format!(
                            "셀 [{r}, {c}]이 격자({}×{}) 밖",
                            self.rows, self.cols
                        ));
                    }
                    out.push((r, c));
                }
            }
            ZoneArea::Polygon(poly) => {
                if poly.len() < 3 {
                    return Err("폴리곤은 점이 3개 이상이어야 함".into());
                }
                for r in 0..self.rows {
                    for c in 0..self.cols {
                        let (x, y) = self.cell_center(r, c);
                        if contains(poly, x, y) {
                            out.push((r, c));
                        }
                    }
                }
            }
        }
        out.sort_unstable();
        out.dedup();
        if out.is_empty() {
            return Err("칠할 셀이 없음".into());
        }
        Ok(out)
    }

    pub fn set(&mut self, cells: &[(u32, u32)], zone: Option<Zone>) {
        for &(r, c) in cells {
            self.cells[(r * self.cols + c) as usize] = zone;
        }
    }

    /// 구역이 지정된 셀 (행·열 순).
    pub fn zoned(&self) -> impl Iterator<Item = (u32, u32, Zone)> + '_ {
        self.cells
            .iter()
            .enumerate()
            .filter_map(move |(i, z)| z.map(|z| (i as u32 / self.cols, i as u32 % self.cols, z)))
    }
}

/// 점이 폴리곤 안에 있는지 (짝홀 규칙).
fn contains(poly: &[Point], x: f64, y: f64) -> bool {
    let mut inside = false;
    let mut j = poly.len() - 1;
    for i in 0..poly.len() {
        let (a, b) = (poly[i], poly[j]);
        if (a.y > y) != (b.y > y) && x < (b.x - a.x) * (y - a.y) / (b.y - a.y) + a.x {
            inside = !inside;
        }
        j = i;
    }
    inside
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn grid_and_polygon() {
        let z = ZoneMap::new(90.0, 90.0, 30.0);
        assert_eq!((z.rows, z.cols), (3, 3));
        assert_eq!(z.cell_center(0, 0), (-30.0, 30.0));
        assert_eq!(z.cell_at(-30.0, 30.0), Some((0, 0)));
        assert_eq!(z.cell_at(100.0, 0.0), None);
        // 북동쪽 2×2 셀 중심을 덮는 사각형
        let poly = ZoneArea::Polygon(vec![
            Point { x: -10.0, y: -10.0 },
            Point { x: 45.0, y: -10.0 },
            Point { x: 45.0, y: 45.0 },
            Point { x: -10.0, y: 45.0 },
        ]);
        assert_eq!(
            z.resolve(&poly).unwrap(),
            vec![(0, 1), (0, 2), (1, 1), (1, 2)]
        );
        assert!(z.resolve(&ZoneArea::Cells(vec![[3, 0]])).is_err());
        assert!(z.resolve(&ZoneArea::Cells(vec![])).is_err());
    }
}
