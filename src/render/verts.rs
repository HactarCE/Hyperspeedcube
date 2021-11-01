use cgmath::Vector3;

#[derive(Debug, Default, Copy, Clone)]
pub struct WireframeVertex {
    pub v0: [f32; 4],
    pub v1: [f32; 4],
    pub v2: [f32; 4],
    pub fill_color: [f32; 4],
    pub wire_color: [f32; 4],
    pub line_mask: [f32; 3],
}
implement_vertex!(
    WireframeVertex,
    v0,
    v1,
    v2,
    fill_color,
    wire_color,
    line_mask,
);
impl WireframeVertex {
    pub fn tri(
        points: [Vector3<f32>; 3],
        line_mask: [bool; 3],
        fill_color: [f32; 4],
        wire_color: [f32; 4],
    ) -> impl Iterator<Item = WireframeVertex> {
        let line_mask = [
            line_mask[0] as u8 as f32,
            line_mask[1] as u8 as f32,
            line_mask[2] as u8 as f32,
        ];
        (0..3).map(move |i| {
            let j = (i + 1) % 3;
            let k = (i + 2) % 3;
            WireframeVertex {
                v0: points[i].extend(1.0).into(),
                v1: points[j].extend(1.0).into(),
                v2: points[k].extend(1.0).into(),
                fill_color,
                wire_color,
                line_mask,
            }
        })
    }
    pub fn quad(
        points: [Vector3<f32>; 4],
        fill_color: [f32; 4],
        wire_color: [f32; 4],
    ) -> impl Iterator<Item = WireframeVertex> {
        let t1 = Self::tri(
            [points[2], points[1], points[0]],
            [true, true, false],
            fill_color,
            wire_color,
        );
        let t2 = Self::tri(
            [points[1], points[2], points[3]],
            [true, true, false],
            fill_color,
            wire_color,
        );
        t1.chain(t2)
    }
    pub fn double_quad(
        [a, b, c, d]: [Vector3<f32>; 4],
        fill_color: [f32; 4],
        wire_color: [f32; 4],
    ) -> impl Iterator<Item = WireframeVertex> {
        let q1 = Self::quad([a, b, c, d], fill_color, wire_color);
        let q2 = Self::quad([a, c, b, d], fill_color, wire_color);
        q1.chain(q2)
    }
    pub fn cube(
        [a, b, c, d, e, f, g, h]: [Vector3<f32>; 8],
        fill_color: [f32; 4],
        wire_color: [f32; 4],
    ) -> impl Iterator<Item = WireframeVertex> {
        let q1 = Self::quad([a, b, c, d], fill_color, wire_color);
        let q2 = Self::quad([e, g, f, h], fill_color, wire_color);
        let q3 = Self::quad([a, c, e, g], fill_color, wire_color);
        let q4 = Self::quad([b, f, d, h], fill_color, wire_color);
        let q5 = Self::quad([a, e, b, f], fill_color, wire_color);
        let q6 = Self::quad([c, d, g, h], fill_color, wire_color);
        q1.chain(q2).chain(q3).chain(q4).chain(q5).chain(q6)
    }

    pub fn avg_z(self) -> f32 {
        (self.v0[3] + self.v1[3] + self.v2[3]) / 3.0
    }
}

#[derive(Debug, Default, Copy, Clone)]
pub struct RgbaVertex {
    pub pos: [f32; 4],
    pub color: [f32; 4],
}
implement_vertex!(RgbaVertex, pos, color);
