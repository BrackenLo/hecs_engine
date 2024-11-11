//====================================================================

use std::fmt::Display;

//====================================================================

#[derive(Clone, Copy, Debug, Hash, PartialEq)]
pub struct Size<T> {
    pub width: T,
    pub height: T,
}

#[allow(dead_code)]
impl<T> Size<T> {
    #[inline]
    pub fn new(width: T, height: T) -> Self {
        Self { width, height }
    }
}

impl<T> From<(T, T)> for Size<T> {
    #[inline]
    fn from(value: (T, T)) -> Self {
        Self {
            width: value.0,
            height: value.1,
        }
    }
}

impl<T> Into<(T, T)> for Size<T> {
    #[inline]
    fn into(self) -> (T, T) {
        (self.width, self.height)
    }
}

impl<T: Display> Display for Size<T> {
    #[inline]
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "({}, {})", self.width, self.height)
    }
}

//====================================================================

#[derive(Clone, Debug, PartialEq)]
pub struct Transform {
    pub translation: glam::Vec3,
    pub rotation: glam::Quat,
    pub scale: glam::Vec3,
}

impl Default for Transform {
    fn default() -> Self {
        Self {
            translation: glam::Vec3::ZERO,
            rotation: glam::Quat::IDENTITY,
            scale: glam::Vec3::ONE,
        }
    }
}

impl Transform {
    #[inline]
    pub fn from_translation(translation: impl Into<glam::Vec3>) -> Self {
        let translation = translation.into();
        Self {
            translation,
            ..Default::default()
        }
    }

    #[inline]
    pub fn from_rotation(rotation: impl Into<glam::Quat>) -> Self {
        let rotation = rotation.into();
        Self {
            rotation,
            ..Default::default()
        }
    }

    #[inline]
    pub fn from_scale(scale: impl Into<glam::Vec3>) -> Self {
        let scale = scale.into();
        Self {
            scale,
            ..Default::default()
        }
    }

    #[inline]
    pub fn from_rotation_translation(
        rotation: impl Into<glam::Quat>,
        translation: impl Into<glam::Vec3>,
    ) -> Self {
        let rotation = rotation.into();
        let translation = translation.into();
        Self {
            translation,
            rotation,
            ..Default::default()
        }
    }

    #[inline]
    pub fn from_scale_translation(
        scale: impl Into<glam::Vec3>,
        translation: impl Into<glam::Vec3>,
    ) -> Self {
        let translation = translation.into();
        let scale = scale.into();
        Self {
            scale,
            translation,
            ..Default::default()
        }
    }

    #[inline]
    pub fn from_scale_rotation(
        scale: impl Into<glam::Vec3>,
        rotation: impl Into<glam::Quat>,
    ) -> Self {
        let scale = scale.into();
        let rotation = rotation.into();
        Self {
            scale,
            rotation,
            ..Default::default()
        }
    }

    #[inline]
    pub fn from_scale_rotation_translation(
        scale: impl Into<glam::Vec3>,
        rotation: impl Into<glam::Quat>,
        translation: impl Into<glam::Vec3>,
    ) -> Self {
        let scale = scale.into();
        let rotation = rotation.into();
        let translation = translation.into();
        Self {
            scale,
            rotation,
            translation,
        }
    }
}

impl Transform {
    pub fn look_to(&mut self, direction: glam::Vec3, up: glam::Vec3) {
        let back = -direction.normalize();
        let right = up
            .cross(back)
            .try_normalize()
            .unwrap_or_else(|| up.any_orthogonal_vector());
        let up = back.cross(right);
        self.rotation = glam::Quat::from_mat3(&glam::Mat3::from_cols(right, up, back));
    }

    #[inline]
    pub fn look_at(&mut self, target: glam::Vec3, up: glam::Vec3) {
        self.look_to(target - self.translation, up);
    }

    #[inline]
    pub fn forward(&self) -> glam::Vec3 {
        self.rotation * glam::Vec3::Z
    }

    #[inline]
    pub fn right(&self) -> glam::Vec3 {
        self.rotation * glam::Vec3::X
    }

    pub fn lerp(&mut self, target: &Transform, s: f32) {
        self.translation = self.translation.lerp(target.translation, s);
        self.rotation = self.rotation.lerp(target.rotation, s);
        self.scale = self.scale.lerp(target.scale, s);
    }
}

impl Transform {
    #[inline]
    pub fn to_matrix(&self) -> glam::Mat4 {
        glam::Mat4::from_scale_rotation_translation(self.scale, self.rotation, self.translation)
    }

    #[inline]
    pub fn to_array(&self) -> [f32; 16] {
        glam::Mat4::from_scale_rotation_translation(self.scale, self.rotation, self.translation)
            .to_cols_array()
    }

    #[inline]
    pub fn to_normal_matrix_array(&self) -> [f32; 9] {
        glam::Mat3::from_quat(self.rotation).to_cols_array()
    }
}

impl Into<glam::Mat4> for &Transform {
    #[inline]
    fn into(self) -> glam::Mat4 {
        glam::Mat4::from_scale_rotation_translation(self.scale, self.rotation, self.translation)
    }
}

//--------------------------------------------------

// TODO - Review these operations
impl std::ops::Add for Transform {
    type Output = Self;

    fn add(mut self, rhs: Transform) -> Self::Output {
        self.translation += rhs.translation;
        self.rotation = self.rotation.mul_quat(rhs.rotation);
        self.scale *= rhs.scale;
        self
    }
}

impl std::ops::AddAssign for Transform {
    fn add_assign(&mut self, rhs: Self) {
        self.translation += rhs.translation;
        self.rotation = self.rotation.mul_quat(rhs.rotation);
        self.scale *= rhs.scale;
    }
}

impl std::ops::Sub for Transform {
    type Output = Self;

    fn sub(mut self, rhs: Self) -> Self::Output {
        self.translation -= rhs.translation;
        self.rotation = self.rotation.mul_quat(rhs.rotation.inverse());
        self.scale /= rhs.scale;

        self
    }
}

//====================================================================
