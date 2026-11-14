//! Small, dependency-free 3D math: [`Vec3`], [`Vec4`], [`Mat4`], [`Quat`] and [`Transform`].
//!
//! Conventions: right-handed coordinates (`+Y` up, the camera looks down `-Z`), column vectors,
//! column-major matrices (`m.cols[c][r]`), OpenGL-style clip space (`-1..1` depth).

use std::ops::{Add, AddAssign, Div, Mul, MulAssign, Neg, Sub, SubAssign};

/// A 3D vector (positions, directions, colors).
#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub struct Vec3 {
    pub x: f32,
    pub y: f32,
    pub z: f32,
}

/// Shorthand for [`Vec3::new`].
#[inline]
pub const fn vec3(x: f32, y: f32, z: f32) -> Vec3 {
    Vec3 { x, y, z }
}

impl Vec3 {
    pub const ZERO: Vec3 = vec3(0.0, 0.0, 0.0);
    pub const ONE: Vec3 = vec3(1.0, 1.0, 1.0);
    pub const X: Vec3 = vec3(1.0, 0.0, 0.0);
    pub const Y: Vec3 = vec3(0.0, 1.0, 0.0);
    pub const Z: Vec3 = vec3(0.0, 0.0, 1.0);

    /// New vector.
    #[inline]
    pub const fn new(x: f32, y: f32, z: f32) -> Self {
        vec3(x, y, z)
    }
    /// All three components set to `v`.
    #[inline]
    pub const fn splat(v: f32) -> Self {
        vec3(v, v, v)
    }
    /// Dot product.
    #[inline]
    pub fn dot(self, o: Vec3) -> f32 {
        self.x * o.x + self.y * o.y + self.z * o.z
    }
    /// Cross product (right-handed).
    #[inline]
    pub fn cross(self, o: Vec3) -> Vec3 {
        vec3(self.y * o.z - self.z * o.y, self.z * o.x - self.x * o.z, self.x * o.y - self.y * o.x)
    }
    /// Squared length.
    #[inline]
    pub fn length_sq(self) -> f32 {
        self.dot(self)
    }
    /// Length.
    #[inline]
    pub fn length(self) -> f32 {
        self.length_sq().sqrt()
    }
    /// Unit vector in the same direction (zero stays zero).
    #[inline]
    pub fn normalized(self) -> Vec3 {
        let l = self.length();
        if l > 1e-12 {
            self / l
        } else {
            Vec3::ZERO
        }
    }
    /// Distance between two points.
    #[inline]
    pub fn distance(self, o: Vec3) -> f32 {
        (self - o).length()
    }
    /// Linear interpolation.
    #[inline]
    pub fn lerp(self, o: Vec3, t: f32) -> Vec3 {
        self + (o - self) * t
    }
    /// Component-wise minimum.
    #[inline]
    pub fn min(self, o: Vec3) -> Vec3 {
        vec3(self.x.min(o.x), self.y.min(o.y), self.z.min(o.z))
    }
    /// Component-wise maximum.
    #[inline]
    pub fn max(self, o: Vec3) -> Vec3 {
        vec3(self.x.max(o.x), self.y.max(o.y), self.z.max(o.z))
    }
    /// Component-wise absolute value.
    #[inline]
    pub fn abs(self) -> Vec3 {
        vec3(self.x.abs(), self.y.abs(), self.z.abs())
    }
    /// Extend to a [`Vec4`].
    #[inline]
    pub fn extend(self, w: f32) -> Vec4 {
        Vec4::new(self.x, self.y, self.z, w)
    }
    /// Any vector perpendicular to this one.
    pub fn any_orthogonal(self) -> Vec3 {
        if self.x.abs() < 0.9 {
            self.cross(Vec3::X).normalized()
        } else {
            self.cross(Vec3::Y).normalized()
        }
    }
    /// Spherical coordinates to a point: `lon` around +Y (radians), `lat` up from the equator.
    pub fn from_spherical(radius: f32, lon: f32, lat: f32) -> Vec3 {
        let c = lat.cos();
        vec3(radius * c * lon.sin(), radius * lat.sin(), radius * c * lon.cos())
    }
}

macro_rules! vec_ops {
    ($t:ident, $($f:ident),+) => {
        impl Add for $t { type Output = $t; #[inline] fn add(self, o: $t) -> $t { $t { $($f: self.$f + o.$f),+ } } }
        impl Sub for $t { type Output = $t; #[inline] fn sub(self, o: $t) -> $t { $t { $($f: self.$f - o.$f),+ } } }
        impl Mul<f32> for $t { type Output = $t; #[inline] fn mul(self, s: f32) -> $t { $t { $($f: self.$f * s),+ } } }
        impl Mul<$t> for f32 { type Output = $t; #[inline] fn mul(self, v: $t) -> $t { v * self } }
        impl Mul<$t> for $t { type Output = $t; #[inline] fn mul(self, o: $t) -> $t { $t { $($f: self.$f * o.$f),+ } } }
        impl Div<f32> for $t { type Output = $t; #[inline] fn div(self, s: f32) -> $t { let k = 1.0 / s; $t { $($f: self.$f * k),+ } } }
        impl Neg for $t { type Output = $t; #[inline] fn neg(self) -> $t { $t { $($f: -self.$f),+ } } }
        impl AddAssign for $t { #[inline] fn add_assign(&mut self, o: $t) { $(self.$f += o.$f;)+ } }
        impl SubAssign for $t { #[inline] fn sub_assign(&mut self, o: $t) { $(self.$f -= o.$f;)+ } }
        impl MulAssign<f32> for $t { #[inline] fn mul_assign(&mut self, s: f32) { $(self.$f *= s;)+ } }
    };
}
vec_ops!(Vec3, x, y, z);
vec_ops!(Vec4, x, y, z, w);

/// A 4D (homogeneous) vector.
#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub struct Vec4 {
    pub x: f32,
    pub y: f32,
    pub z: f32,
    pub w: f32,
}

impl Vec4 {
    /// New vector.
    #[inline]
    pub const fn new(x: f32, y: f32, z: f32, w: f32) -> Self {
        Self { x, y, z, w }
    }
    /// Drop `w`.
    #[inline]
    pub fn xyz(self) -> Vec3 {
        vec3(self.x, self.y, self.z)
    }
    /// Dot product.
    #[inline]
    pub fn dot(self, o: Vec4) -> f32 {
        self.x * o.x + self.y * o.y + self.z * o.z + self.w * o.w
    }
    /// Perspective divide (`xyz / w`).
    #[inline]
    pub fn project(self) -> Vec3 {
        self.xyz() / self.w
    }
}

/// A 4x4 column-major matrix.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Mat4 {
    /// `cols[c][r]`.
    pub cols: [[f32; 4]; 4],
}

impl Default for Mat4 {
    fn default() -> Self {
        Self::IDENTITY
    }
}

impl Mat4 {
    pub const IDENTITY: Mat4 =
        Mat4 { cols: [[1.0, 0.0, 0.0, 0.0], [0.0, 1.0, 0.0, 0.0], [0.0, 0.0, 1.0, 0.0], [0.0, 0.0, 0.0, 1.0]] };

    /// Build from four column vectors.
    pub fn from_cols(x: Vec4, y: Vec4, z: Vec4, w: Vec4) -> Self {
        Self { cols: [[x.x, x.y, x.z, x.w], [y.x, y.y, y.z, y.w], [z.x, z.y, z.z, z.w], [w.x, w.y, w.z, w.w]] }
    }
    /// Element at row `r`, column `c`.
    #[inline]
    pub fn at(&self, r: usize, c: usize) -> f32 {
        self.cols[c][r]
    }
    /// Translation matrix.
    pub fn translation(t: Vec3) -> Self {
        let mut m = Self::IDENTITY;
        m.cols[3] = [t.x, t.y, t.z, 1.0];
        m
    }
    /// Non-uniform scale matrix.
    pub fn scale(s: Vec3) -> Self {
        let mut m = Self::IDENTITY;
        m.cols[0][0] = s.x;
        m.cols[1][1] = s.y;
        m.cols[2][2] = s.z;
        m
    }
    /// Rotation around +X by `a` radians.
    pub fn rotation_x(a: f32) -> Self {
        let (s, c) = a.sin_cos();
        let mut m = Self::IDENTITY;
        m.cols[1] = [0.0, c, s, 0.0];
        m.cols[2] = [0.0, -s, c, 0.0];
        m
    }
    /// Rotation around +Y by `a` radians.
    pub fn rotation_y(a: f32) -> Self {
        let (s, c) = a.sin_cos();
        let mut m = Self::IDENTITY;
        m.cols[0] = [c, 0.0, -s, 0.0];
        m.cols[2] = [s, 0.0, c, 0.0];
        m
    }
    /// Rotation around +Z by `a` radians.
    pub fn rotation_z(a: f32) -> Self {
        let (s, c) = a.sin_cos();
        let mut m = Self::IDENTITY;
        m.cols[0] = [c, s, 0.0, 0.0];
        m.cols[1] = [-s, c, 0.0, 0.0];
        m
    }
    /// Rotation around an arbitrary axis.
    pub fn rotation(axis: Vec3, angle: f32) -> Self {
        Quat::from_axis_angle(axis, angle).to_mat4()
    }
    /// Right-handed perspective projection (`fov_y` in radians), OpenGL depth range.
    pub fn perspective(fov_y: f32, aspect: f32, near: f32, far: f32) -> Self {
        let f = 1.0 / (fov_y / 2.0).tan();
        let nf = 1.0 / (near - far);
        Self {
            cols: [
                [f / aspect, 0.0, 0.0, 0.0],
                [0.0, f, 0.0, 0.0],
                [0.0, 0.0, (far + near) * nf, -1.0],
                [0.0, 0.0, 2.0 * far * near * nf, 0.0],
            ],
        }
    }
    /// Right-handed orthographic projection.
    pub fn orthographic(left: f32, right: f32, bottom: f32, top: f32, near: f32, far: f32) -> Self {
        let (rl, tb, fnr) = (right - left, top - bottom, far - near);
        Self {
            cols: [
                [2.0 / rl, 0.0, 0.0, 0.0],
                [0.0, 2.0 / tb, 0.0, 0.0],
                [0.0, 0.0, -2.0 / fnr, 0.0],
                [-(right + left) / rl, -(top + bottom) / tb, -(far + near) / fnr, 1.0],
            ],
        }
    }
    /// Right-handed view matrix looking from `eye` at `target`.
    pub fn look_at(eye: Vec3, target: Vec3, up: Vec3) -> Self {
        let f = (target - eye).normalized();
        let mut s = f.cross(up).normalized();
        if s.length_sq() < 1e-8 {
            s = f.any_orthogonal();
        }
        let u = s.cross(f);
        Self {
            cols: [
                [s.x, u.x, -f.x, 0.0],
                [s.y, u.y, -f.y, 0.0],
                [s.z, u.z, -f.z, 0.0],
                [-s.dot(eye), -u.dot(eye), f.dot(eye), 1.0],
            ],
        }
    }
    /// Transpose.
    pub fn transpose(&self) -> Self {
        let mut m = Self::IDENTITY;
        for c in 0..4 {
            for r in 0..4 {
                m.cols[c][r] = self.cols[r][c];
            }
        }
        m
    }
    /// Determinant.
    pub fn determinant(&self) -> f32 {
        let m = |r: usize, c: usize| self.cols[c][r];
        let s0 = m(0, 0) * m(1, 1) - m(1, 0) * m(0, 1);
        let s1 = m(0, 0) * m(1, 2) - m(1, 0) * m(0, 2);
        let s2 = m(0, 0) * m(1, 3) - m(1, 0) * m(0, 3);
        let s3 = m(0, 1) * m(1, 2) - m(1, 1) * m(0, 2);
        let s4 = m(0, 1) * m(1, 3) - m(1, 1) * m(0, 3);
        let s5 = m(0, 2) * m(1, 3) - m(1, 2) * m(0, 3);
        let c5 = m(2, 2) * m(3, 3) - m(3, 2) * m(2, 3);
        let c4 = m(2, 1) * m(3, 3) - m(3, 1) * m(2, 3);
        let c3 = m(2, 1) * m(3, 2) - m(3, 1) * m(2, 2);
        let c2 = m(2, 0) * m(3, 3) - m(3, 0) * m(2, 3);
        let c1 = m(2, 0) * m(3, 2) - m(3, 0) * m(2, 2);
        let c0 = m(2, 0) * m(3, 1) - m(3, 0) * m(2, 1);
        s0 * c5 - s1 * c4 + s2 * c3 + s3 * c2 - s4 * c1 + s5 * c0
    }
    /// General inverse. Returns `None` for singular matrices.
    pub fn inverse(&self) -> Option<Self> {
        let m = |r: usize, c: usize| self.cols[c][r];
        let s0 = m(0, 0) * m(1, 1) - m(1, 0) * m(0, 1);
        let s1 = m(0, 0) * m(1, 2) - m(1, 0) * m(0, 2);
        let s2 = m(0, 0) * m(1, 3) - m(1, 0) * m(0, 3);
        let s3 = m(0, 1) * m(1, 2) - m(1, 1) * m(0, 2);
        let s4 = m(0, 1) * m(1, 3) - m(1, 1) * m(0, 3);
        let s5 = m(0, 2) * m(1, 3) - m(1, 2) * m(0, 3);
        let c5 = m(2, 2) * m(3, 3) - m(3, 2) * m(2, 3);
        let c4 = m(2, 1) * m(3, 3) - m(3, 1) * m(2, 3);
        let c3 = m(2, 1) * m(3, 2) - m(3, 1) * m(2, 2);
        let c2 = m(2, 0) * m(3, 3) - m(3, 0) * m(2, 3);
        let c1 = m(2, 0) * m(3, 2) - m(3, 0) * m(2, 2);
        let c0 = m(2, 0) * m(3, 1) - m(3, 0) * m(2, 1);
        let det = s0 * c5 - s1 * c4 + s2 * c3 + s3 * c2 - s4 * c1 + s5 * c0;
        if det.abs() < 1e-12 {
            return None;
        }
        let k = 1.0 / det;
        let mut r = [[0.0f32; 4]; 4]; // r[row][col]
        r[0][0] = (m(1, 1) * c5 - m(1, 2) * c4 + m(1, 3) * c3) * k;
        r[0][1] = (-m(0, 1) * c5 + m(0, 2) * c4 - m(0, 3) * c3) * k;
        r[0][2] = (m(3, 1) * s5 - m(3, 2) * s4 + m(3, 3) * s3) * k;
        r[0][3] = (-m(2, 1) * s5 + m(2, 2) * s4 - m(2, 3) * s3) * k;
        r[1][0] = (-m(1, 0) * c5 + m(1, 2) * c2 - m(1, 3) * c1) * k;
        r[1][1] = (m(0, 0) * c5 - m(0, 2) * c2 + m(0, 3) * c1) * k;
        r[1][2] = (-m(3, 0) * s5 + m(3, 2) * s2 - m(3, 3) * s1) * k;
        r[1][3] = (m(2, 0) * s5 - m(2, 2) * s2 + m(2, 3) * s1) * k;
        r[2][0] = (m(1, 0) * c4 - m(1, 1) * c2 + m(1, 3) * c0) * k;
        r[2][1] = (-m(0, 0) * c4 + m(0, 1) * c2 - m(0, 3) * c0) * k;
        r[2][2] = (m(3, 0) * s4 - m(3, 1) * s2 + m(3, 3) * s0) * k;
        r[2][3] = (-m(2, 0) * s4 + m(2, 1) * s2 - m(2, 3) * s0) * k;
        r[3][0] = (-m(1, 0) * c3 + m(1, 1) * c1 - m(1, 2) * c0) * k;
        r[3][1] = (m(0, 0) * c3 - m(0, 1) * c1 + m(0, 2) * c0) * k;
        r[3][2] = (-m(3, 0) * s3 + m(3, 1) * s1 - m(3, 2) * s0) * k;
        r[3][3] = (m(2, 0) * s3 - m(2, 1) * s1 + m(2, 2) * s0) * k;
        let mut out = Self::IDENTITY;
        for (row, vals) in r.iter().enumerate() {
            for (col, v) in vals.iter().enumerate() {
                out.cols[col][row] = *v;
            }
        }
        Some(out)
    }
    /// Multiply a 4D vector.
    #[inline]
    pub fn mul_vec4(&self, v: Vec4) -> Vec4 {
        let c = &self.cols;
        Vec4::new(
            c[0][0] * v.x + c[1][0] * v.y + c[2][0] * v.z + c[3][0] * v.w,
            c[0][1] * v.x + c[1][1] * v.y + c[2][1] * v.z + c[3][1] * v.w,
            c[0][2] * v.x + c[1][2] * v.y + c[2][2] * v.z + c[3][2] * v.w,
            c[0][3] * v.x + c[1][3] * v.y + c[2][3] * v.z + c[3][3] * v.w,
        )
    }
    /// Transform a point (`w = 1`, affine: no perspective divide).
    #[inline]
    pub fn transform_point(&self, p: Vec3) -> Vec3 {
        let c = &self.cols;
        vec3(
            c[0][0] * p.x + c[1][0] * p.y + c[2][0] * p.z + c[3][0],
            c[0][1] * p.x + c[1][1] * p.y + c[2][1] * p.z + c[3][1],
            c[0][2] * p.x + c[1][2] * p.y + c[2][2] * p.z + c[3][2],
        )
    }
    /// Transform a direction (`w = 0`).
    #[inline]
    pub fn transform_vector(&self, v: Vec3) -> Vec3 {
        let c = &self.cols;
        vec3(
            c[0][0] * v.x + c[1][0] * v.y + c[2][0] * v.z,
            c[0][1] * v.x + c[1][1] * v.y + c[2][1] * v.z,
            c[0][2] * v.x + c[1][2] * v.y + c[2][2] * v.z,
        )
    }
    /// Transform a point and perform the perspective divide.
    pub fn project_point(&self, p: Vec3) -> Vec3 {
        self.mul_vec4(p.extend(1.0)).project()
    }
    /// The matrix that transforms normals (inverse transpose of the upper 3x3).
    pub fn normal_matrix(&self) -> Mat4 {
        let mut m = *self;
        m.cols[3] = [0.0, 0.0, 0.0, 1.0];
        m.cols[0][3] = 0.0;
        m.cols[1][3] = 0.0;
        m.cols[2][3] = 0.0;
        m.inverse().map(|i| i.transpose()).unwrap_or(Mat4::IDENTITY)
    }
}

impl Mul for Mat4 {
    type Output = Mat4;
    fn mul(self, o: Mat4) -> Mat4 {
        let mut out = [[0.0f32; 4]; 4];
        for (c, col) in out.iter_mut().enumerate() {
            let v = self.mul_vec4(Vec4::new(o.cols[c][0], o.cols[c][1], o.cols[c][2], o.cols[c][3]));
            *col = [v.x, v.y, v.z, v.w];
        }
        Mat4 { cols: out }
    }
}

impl Mul<Vec4> for Mat4 {
    type Output = Vec4;
    fn mul(self, v: Vec4) -> Vec4 {
        self.mul_vec4(v)
    }
}

/// A rotation quaternion (`x, y, z` imaginary, `w` real).
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Quat {
    pub x: f32,
    pub y: f32,
    pub z: f32,
    pub w: f32,
}

impl Default for Quat {
    fn default() -> Self {
        Self::IDENTITY
    }
}

impl Quat {
    pub const IDENTITY: Quat = Quat { x: 0.0, y: 0.0, z: 0.0, w: 1.0 };

    /// Rotation of `angle` radians around `axis` (need not be normalized).
    pub fn from_axis_angle(axis: Vec3, angle: f32) -> Self {
        let a = axis.normalized();
        let (s, c) = (angle / 2.0).sin_cos();
        Quat { x: a.x * s, y: a.y * s, z: a.z * s, w: c }
    }
    /// Rotation from Euler angles, applied in order X (pitch), then Y (yaw), then Z (roll).
    pub fn from_euler(x: f32, y: f32, z: f32) -> Self {
        Quat::from_axis_angle(Vec3::Z, z) * Quat::from_axis_angle(Vec3::Y, y) * Quat::from_axis_angle(Vec3::X, x)
    }
    /// Shortest rotation taking direction `from` to direction `to`.
    pub fn from_rotation_arc(from: Vec3, to: Vec3) -> Self {
        let (f, t) = (from.normalized(), to.normalized());
        let d = f.dot(t);
        if d < -0.99999 {
            return Quat::from_axis_angle(f.any_orthogonal(), std::f32::consts::PI);
        }
        let c = f.cross(t);
        Quat { x: c.x, y: c.y, z: c.z, w: 1.0 + d }.normalized()
    }
    /// Unit-length copy.
    pub fn normalized(self) -> Self {
        let l = (self.x * self.x + self.y * self.y + self.z * self.z + self.w * self.w).sqrt();
        if l < 1e-12 {
            return Self::IDENTITY;
        }
        Quat { x: self.x / l, y: self.y / l, z: self.z / l, w: self.w / l }
    }
    /// The inverse rotation (for unit quaternions).
    pub fn conjugate(self) -> Self {
        Quat { x: -self.x, y: -self.y, z: -self.z, w: self.w }
    }
    /// 4D dot product.
    pub fn dot(self, o: Quat) -> f32 {
        self.x * o.x + self.y * o.y + self.z * o.z + self.w * o.w
    }
    /// Rotate a vector.
    #[inline]
    pub fn rotate(self, v: Vec3) -> Vec3 {
        let q = vec3(self.x, self.y, self.z);
        let t = q.cross(v) * 2.0;
        v + t * self.w + q.cross(t)
    }
    /// Spherical linear interpolation along the shortest arc.
    pub fn slerp(self, o: Quat, t: f32) -> Quat {
        let mut d = self.dot(o);
        let mut b = o;
        if d < 0.0 {
            d = -d;
            b = Quat { x: -o.x, y: -o.y, z: -o.z, w: -o.w };
        }
        if d > 0.9995 {
            let l = |a: f32, b: f32| a + (b - a) * t;
            return Quat { x: l(self.x, b.x), y: l(self.y, b.y), z: l(self.z, b.z), w: l(self.w, b.w) }.normalized();
        }
        let th = d.clamp(-1.0, 1.0).acos();
        let s = th.sin();
        let (wa, wb) = (((1.0 - t) * th).sin() / s, (t * th).sin() / s);
        Quat {
            x: self.x * wa + b.x * wb,
            y: self.y * wa + b.y * wb,
            z: self.z * wa + b.z * wb,
            w: self.w * wa + b.w * wb,
        }
    }
    /// Rotation matrix.
    pub fn to_mat4(self) -> Mat4 {
        let Quat { x, y, z, w } = self;
        let (x2, y2, z2) = (x + x, y + y, z + z);
        let (xx, yy, zz) = (x * x2, y * y2, z * z2);
        let (xy, xz, yz) = (x * y2, x * z2, y * z2);
        let (wx, wy, wz) = (w * x2, w * y2, w * z2);
        Mat4 {
            cols: [
                [1.0 - (yy + zz), xy + wz, xz - wy, 0.0],
                [xy - wz, 1.0 - (xx + zz), yz + wx, 0.0],
                [xz + wy, yz - wx, 1.0 - (xx + yy), 0.0],
                [0.0, 0.0, 0.0, 1.0],
            ],
        }
    }
}

impl Mul for Quat {
    type Output = Quat;
    /// `a * b` applies `b` first, then `a`.
    fn mul(self, b: Quat) -> Quat {
        let a = self;
        Quat {
            w: a.w * b.w - a.x * b.x - a.y * b.y - a.z * b.z,
            x: a.w * b.x + a.x * b.w + a.y * b.z - a.z * b.y,
            y: a.w * b.y - a.x * b.z + a.y * b.w + a.z * b.x,
            z: a.w * b.z + a.x * b.y - a.y * b.x + a.z * b.w,
        }
    }
}

/// Translation + rotation + scale: where an object sits in the world.
///
/// ```no_run
/// use glowkit_3d::{Transform, Quat, vec3};
/// let t = Transform::from_translation(vec3(0.0, 1.0, 0.0))
///     .with_rotation(Quat::from_euler(0.3, 1.2, 0.0))
///     .with_scale(0.5);
/// ```
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Transform {
    pub translation: Vec3,
    pub rotation: Quat,
    pub scale: Vec3,
}

impl Default for Transform {
    fn default() -> Self {
        Self::IDENTITY
    }
}

impl Transform {
    pub const IDENTITY: Transform = Transform { translation: Vec3::ZERO, rotation: Quat::IDENTITY, scale: Vec3::ONE };

    /// Just a translation.
    pub fn from_translation(t: Vec3) -> Self {
        Self { translation: t, ..Self::IDENTITY }
    }
    /// Set the rotation.
    pub fn with_rotation(mut self, q: Quat) -> Self {
        self.rotation = q;
        self
    }
    /// Set the rotation from Euler angles (radians).
    pub fn with_euler(self, x: f32, y: f32, z: f32) -> Self {
        self.with_rotation(Quat::from_euler(x, y, z))
    }
    /// Uniform scale.
    pub fn with_scale(mut self, s: f32) -> Self {
        self.scale = Vec3::splat(s);
        self
    }
    /// Non-uniform scale.
    pub fn with_scale3(mut self, s: Vec3) -> Self {
        self.scale = s;
        self
    }
    /// Set the translation.
    pub fn with_translation(mut self, t: Vec3) -> Self {
        self.translation = t;
        self
    }
    /// The model matrix `T * R * S`.
    pub fn matrix(&self) -> Mat4 {
        let mut m = self.rotation.to_mat4();
        for (c, s) in [self.scale.x, self.scale.y, self.scale.z].into_iter().enumerate() {
            for r in 0..3 {
                m.cols[c][r] *= s;
            }
        }
        m.cols[3] = [self.translation.x, self.translation.y, self.translation.z, 1.0];
        m
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::f32::consts::{FRAC_PI_2, PI};

    fn close(a: Vec3, b: Vec3) -> bool {
        (a - b).length() < 1e-4
    }

    #[test]
    fn vec_basics() {
        assert_eq!(Vec3::X.cross(Vec3::Y), Vec3::Z);
        assert_eq!(vec3(1.0, 2.0, 3.0).dot(vec3(4.0, 5.0, 6.0)), 32.0);
        assert!((vec3(3.0, 4.0, 0.0).length() - 5.0).abs() < 1e-6);
        assert!((vec3(0.0, 0.0, 9.0).normalized() - Vec3::Z).length() < 1e-6);
        assert_eq!(Vec3::ZERO.normalized(), Vec3::ZERO);
    }

    #[test]
    fn rotations_agree() {
        let v = vec3(1.0, 2.0, 3.0);
        for (axis, m) in
            [(Vec3::X, Mat4::rotation_x(0.7)), (Vec3::Y, Mat4::rotation_y(0.7)), (Vec3::Z, Mat4::rotation_z(0.7))]
        {
            let q = Quat::from_axis_angle(axis, 0.7);
            assert!(close(q.rotate(v), m.transform_point(v)));
            assert!(close(q.to_mat4().transform_point(v), m.transform_point(v)));
        }
        assert!(close(Quat::from_axis_angle(Vec3::Y, FRAC_PI_2).rotate(Vec3::X), -Vec3::Z));
    }

    #[test]
    fn inverse_roundtrip() {
        let m = Mat4::translation(vec3(1.0, -2.0, 3.0))
            * Mat4::rotation(vec3(1.0, 1.0, 0.0), 0.9)
            * Mat4::scale(vec3(2.0, 0.5, 1.5));
        let inv = m.inverse().unwrap();
        let id = m * inv;
        for c in 0..4 {
            for r in 0..4 {
                let e = if c == r { 1.0 } else { 0.0 };
                assert!((id.cols[c][r] - e).abs() < 1e-4);
            }
        }
        assert!((m.determinant() - 1.5).abs() < 1e-3);
        assert!(Mat4::scale(vec3(1.0, 0.0, 1.0)).inverse().is_none());
    }

    #[test]
    fn look_at_and_perspective() {
        let v = Mat4::look_at(vec3(0.0, 0.0, 5.0), Vec3::ZERO, Vec3::Y);
        assert!(close(v.transform_point(Vec3::ZERO), vec3(0.0, 0.0, -5.0)));
        let p = Mat4::perspective(PI / 2.0, 1.0, 1.0, 10.0);
        let near = p.project_point(vec3(0.0, 0.0, -1.0));
        let far = p.project_point(vec3(0.0, 0.0, -10.0));
        assert!((near.z + 1.0).abs() < 1e-4 && (far.z - 1.0).abs() < 1e-4);
        let edge = p.project_point(vec3(2.0, 0.0, -2.0));
        assert!((edge.x - 1.0).abs() < 1e-4);
        let o = Mat4::orthographic(-2.0, 2.0, -1.0, 1.0, 0.1, 10.0);
        assert!(close(o.project_point(vec3(2.0, 1.0, -0.1)), vec3(1.0, 1.0, -1.0)));
    }

    #[test]
    fn slerp_midpoint() {
        let a = Quat::IDENTITY;
        let b = Quat::from_axis_angle(Vec3::Z, FRAC_PI_2);
        let m = a.slerp(b, 0.5);
        assert!(close(m.rotate(Vec3::X), vec3(0.5f32.sqrt(), 0.5f32.sqrt(), 0.0)));
        assert!(close(a.slerp(b, 1.0).rotate(Vec3::X), Vec3::Y));
        let arc = Quat::from_rotation_arc(Vec3::X, Vec3::Z);
        assert!(close(arc.rotate(Vec3::X), Vec3::Z));
    }

    #[test]
    fn transform_matrix() {
        let t = Transform::from_translation(vec3(1.0, 0.0, 0.0))
            .with_rotation(Quat::from_axis_angle(Vec3::Z, FRAC_PI_2))
            .with_scale(2.0);
        assert!(close(t.matrix().transform_point(Vec3::X), vec3(1.0, 2.0, 0.0)));
        let n = t.matrix().normal_matrix().transform_vector(Vec3::X).normalized();
        assert!(close(n, Vec3::Y));
    }
}
