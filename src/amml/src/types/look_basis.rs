// the view rotation of a camera at the origin that looks along yaw and pitch: the basis vectors and the view matrix
// as glm::lookAt builds it (right handed, column major), and its inverse
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct LookBasis {
    pub forward: [f32; 3],
    pub right: [f32; 3],
    pub up: [f32; 3],
    pub view: [[f32; 4]; 4],
    pub inverse: [[f32; 4]; 4],
}

impl LookBasis {
    const WORLD_UP: [f32; 3] = [0.0, 1.0, 0.0];

    // yaw and pitch in degrees; pitch positive looks down
    pub fn from_angles(yaw: f32, pitch: f32) -> Self {
        let (sin_yaw, cos_yaw) = yaw.to_radians().sin_cos();
        let (sin_pitch, cos_pitch) = pitch.to_radians().sin_cos();
        let forward = [-sin_yaw * cos_pitch, -sin_pitch, cos_yaw * cos_pitch];
        let right = Self::normalized(Self::cross(forward, Self::WORLD_UP));
        let up = Self::cross(right, forward);
        let f = Self::normalized(forward);
        let mut view = [[0.0f32; 4]; 4];
        for axis in 0..3 {
            view[axis][0] = right[axis];
            view[axis][1] = up[axis];
            view[axis][2] = -f[axis];
        }
        view[3][3] = 1.0;
        // the eye is at the origin, so the view is a pure rotation and its inverse is its transpose
        let mut inverse = [[0.0f32; 4]; 4];
        for (column, values) in inverse.iter_mut().enumerate() {
            for (row, value) in values.iter_mut().enumerate() {
                *value = view[row][column];
            }
        }
        Self { forward, right, up, view, inverse }
    }

    fn cross(a: [f32; 3], b: [f32; 3]) -> [f32; 3] {
        [a[1] * b[2] - a[2] * b[1], a[2] * b[0] - a[0] * b[2], a[0] * b[1] - a[1] * b[0]]
    }

    fn normalized(v: [f32; 3]) -> [f32; 3] {
        let length = (v[0] * v[0] + v[1] * v[1] + v[2] * v[2]).sqrt();
        [v[0] / length, v[1] / length, v[2] / length]
    }
}
