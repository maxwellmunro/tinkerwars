use bincode::{Decode, Encode};
use rapier2d::math::Point;

#[derive(Clone, Copy, Debug, Default, Encode, Decode)]
pub(crate) struct Vec2 {
    pub x: f32,
    pub y: f32,
}

impl From<Vec2> for sdl2::rect::Point {
    fn from(value: Vec2) -> Self {
        sdl2::rect::Point::new(value.x as i32, value.y as i32)
    }
}

impl From<rapier2d::math::Point<f32>> for Vec2 {
    fn from(value: rapier2d::math::Point<f32>) -> Self {
        Vec2 {
            x: value.x,
            y: value.y,
        }
    }
}

impl Vec2 {
    pub(crate) fn add(self, rhs: Vec2) -> Self {
        Vec2 {
            x: self.x + rhs.x,
            y: self.y + rhs.y,
        }
    }

    pub(crate) fn mul(self, rhs: f32) -> Self {
        Vec2 {
            x: self.x * rhs,
            y: self.y * rhs,
        }
    }
}

type Polygon = Vec<Point<f32>>;

pub fn translate_polygon(polygon: &mut Polygon, dx: f32, dy: f32) {
    polygon
        .iter_mut()
        .for_each(|p| *p = Point::new(p.x + dx, p.y + dy));
}

pub fn scale_polygon(polygon: &mut Polygon, scale: f32) {
    polygon
        .iter_mut()
        .for_each(|p| *p = Point::new(p.x * scale, p.y * scale));
}

pub fn rotate_polygon(polygon: &mut Polygon, angle: f32) {
    polygon.iter_mut().for_each(|p| {
        let angle = p.y.atan2(p.x) + angle;
        let mag = (p.x * p.x + p.y * p.y).sqrt();

        *p = Point::new(mag * angle.cos(), mag * angle.sin());
    })
}

pub fn point_intersects_polygon(p: Point<f32>, polygon: &Polygon) -> bool {
    let mut pairs = polygon.windows(2).collect::<Vec<&[Point<f32>]>>();
    let binding = vec![polygon[0], *polygon.last().unwrap()];
    pairs.push(binding.as_slice());

    let (x, y) = (p.x, p.y);

    pairs
        .iter()
        .map(|l| {
            let (x1, y1) = (l[0].x, l[0].y);
            let (x2, y2) = (l[1].x, l[1].y);

            if ((y1 > y) != (y2 > y)) && (x < (x2 - x1) * (y - y1) / (y2 - y1 + f32::EPSILON) + x1)
            {
                1
            } else {
                0
            }
        })
        .sum::<u32>()
        % 2
        == 1
}
fn orientation(p: Point<f32>, q: Point<f32>, r: Point<f32>) -> i32 {
    let val = (q.y - p.y) * (r.x - q.x) - (q.x - p.x) * (r.y - q.y);
    if val.abs() < f32::EPSILON {
        0
    } else if val > 0.0 {
        1
    } else {
        2
    }
}

fn on_segment(p: Point<f32>, q: Point<f32>, r: Point<f32>) -> bool {
    q.x <= p.x.max(r.x) && q.x >= p.x.min(r.x) && q.y <= p.y.max(r.y) && q.y >= p.y.min(r.y)
}

pub fn lines_intersect(a: &[Point<f32>], b: &[Point<f32>]) -> bool {
    let (p1, q1) = (a[0], a[1]);
    let (p2, q2) = (b[0], b[1]);

    let o1 = orientation(p1, q1, p2);
    let o2 = orientation(p1, q1, q2);
    let o3 = orientation(p2, q2, p1);
    let o4 = orientation(p2, q2, q1);

    if o1 != o2 && o3 != o4 {
        return true;
    }

    (o1 == 0 && on_segment(p1, p2, q1))
        || (o2 == 0 && on_segment(p1, q2, q1))
        || (o3 == 0 && on_segment(p2, p1, q2))
        || (o4 == 0 && on_segment(p2, q1, q2))
}

pub fn polygons_intersect(a: &Polygon, b: &Polygon) -> bool {
    if a.iter().any(|&p| point_intersects_polygon(p, b)) {
        return true;
    }

    if b.iter().any(|&p| point_intersects_polygon(p, a)) {
        return true;
    }

    for i in 0..a.len() {
        let l_a = &[a[i], a[(i + 1) % a.len()]];
        for j in 0..b.len() {
            let l_b = &[b[j], b[(j + 1) % b.len()]];
            if lines_intersect(l_a, l_b) {
                return true;
            }
        }
    }

    false
}
