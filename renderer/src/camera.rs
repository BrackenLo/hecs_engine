//====================================================================

use common::Transform;
use hecs::World;

use crate::WgpuWrapper;

//====================================================================

pub(crate) fn sys_prep_perspective_cameras(world: &mut World, queue: &wgpu::Queue) {
    world
        .query_mut::<(&CameraWgpu, &PerspectiveCamera, &Transform)>()
        .into_iter()
        .for_each(|(_, (camera, perspective, transform))| {
            camera.update_camera(queue, perspective, transform)
        });
}

pub(crate) fn sys_prep_orthographic_cameras(world: &mut World, queue: &wgpu::Queue) {
    world
        .query_mut::<(&CameraWgpu, &OrthographicCamera, &Transform)>()
        .into_iter()
        .for_each(|(_, (camera, orthographic, transform))| {
            camera.update_camera(queue, orthographic, transform)
        });
}

//====================================================================

pub struct CameraWgpu {
    pub(crate) camera_buffer: WgpuWrapper<wgpu::Buffer>,
    pub(crate) camera_bind_group: WgpuWrapper<wgpu::BindGroup>,
}

impl CameraWgpu {
    #[inline]
    pub fn update_camera<C: CameraUniform>(
        &self,
        queue: &wgpu::Queue,
        camera: &C,
        transform: &Transform,
    ) {
        queue
            .write_buffer_with(
                self.camera_buffer.inner(),
                0,
                wgpu::BufferSize::new(std::mem::size_of::<CameraUniformRaw>() as u64).unwrap(),
            )
            .unwrap()
            .copy_from_slice(bytemuck::cast_slice(
                &[camera.get_camera_uniform(transform)],
            ));
    }

    #[inline]
    pub fn bind_group(&self) -> &wgpu::BindGroup {
        self.camera_bind_group.inner()
    }
}

//====================================================================

pub trait CameraUniform {
    fn get_projection_matrix(&self) -> glam::Mat4;
    fn get_view_matrix(&self, transform: &Transform) -> glam::Mat4;

    #[inline]
    fn get_camera_uniform(&self, transform: &Transform) -> CameraUniformRaw {
        CameraUniformRaw::new(
            self.get_projection_matrix() * self.get_view_matrix(transform),
            transform.translation,
        )
    }
}

#[repr(C)]
#[derive(bytemuck::Pod, bytemuck::Zeroable, Clone, Copy)]
pub struct CameraUniformRaw {
    view_projection: glam::Mat4,
    camera_position: glam::Vec3,
    _padding: u32,
}

impl CameraUniformRaw {
    pub fn new(view_projection: glam::Mat4, camera_position: glam::Vec3) -> Self {
        Self {
            view_projection,
            camera_position,
            _padding: 0,
        }
    }
}

//--------------------------------------------------

#[derive(Debug, Clone, PartialEq)]
pub struct OrthographicCamera {
    pub left: f32,
    pub right: f32,
    pub bottom: f32,
    pub top: f32,
    pub z_near: f32,
    pub z_far: f32,
    // pub translation: glam::Vec3,
    // pub rotation: glam::Quat,
}

impl Default for OrthographicCamera {
    fn default() -> Self {
        Self {
            left: 0.,
            right: 1920.,
            bottom: 0.,
            top: 1080.,
            z_near: 0.,
            z_far: 1000000.,
            // translation: glam::Vec3::ZERO,
            // rotation: glam::Quat::IDENTITY,
        }
    }
}

impl CameraUniform for OrthographicCamera {
    #[inline]
    fn get_projection_matrix(&self) -> glam::Mat4 {
        self.get_projection()
    }

    #[inline]
    fn get_view_matrix(&self, transform: &Transform) -> glam::Mat4 {
        glam::Mat4::from_rotation_translation(transform.rotation, -transform.translation)
    }
}

impl OrthographicCamera {
    fn get_projection(&self) -> glam::Mat4 {
        let projection_matrix = glam::Mat4::orthographic_lh(
            self.left,
            self.right,
            self.bottom,
            self.top,
            self.z_near,
            self.z_far,
        );

        projection_matrix

        // let transform_matrix =
        //     glam::Mat4::from_rotation_translation(self.rotation, -self.translation);

        // projection_matrix * transform_matrix
    }

    pub fn new_sized(width: f32, height: f32) -> Self {
        Self {
            left: 0.,
            right: width,
            bottom: 0.,
            top: height,
            ..Default::default()
        }
    }

    pub fn new_centered(half_width: f32, half_height: f32) -> Self {
        Self {
            left: -half_width,
            right: half_width,
            bottom: -half_height,
            top: half_height,
            ..Default::default()
        }
    }

    pub fn set_size(&mut self, width: f32, height: f32) {
        let half_width = width / 2.;
        let half_height = height / 2.;

        self.left = -half_width;
        self.right = half_width;
        self.top = half_height;
        self.bottom = -half_height;
    }
}

//--------------------------------------------------

#[derive(Debug, Clone, PartialEq)]
pub struct PerspectiveCamera {
    pub up: glam::Vec3,
    pub aspect: f32,
    pub fovy: f32,
    pub z_near: f32,
    pub z_far: f32,
    // pub translation: glam::Vec3,
    // pub rotation: glam::Quat,
}

impl Default for PerspectiveCamera {
    fn default() -> Self {
        Self {
            up: glam::Vec3::Y,
            aspect: 1.7777777778,
            fovy: 45.,
            z_near: 0.1,
            z_far: 1000000.,
            // translation: glam::Vec3::ZERO,
            // rotation: glam::Quat::IDENTITY,
        }
    }
}

impl CameraUniform for PerspectiveCamera {
    #[inline]
    fn get_projection_matrix(&self) -> glam::Mat4 {
        self.get_projection()
        // CameraUniformRaw::new(self.get_projection(), self.translation.into())
    }

    fn get_view_matrix(&self, transform: &Transform) -> glam::Mat4 {
        let forward = transform.forward();

        glam::Mat4::look_at_lh(
            transform.translation,
            transform.translation + forward,
            self.up,
        )
    }
}

impl PerspectiveCamera {
    fn get_projection(&self) -> glam::Mat4 {
        // let forward = (self.rotation * glam::Vec3::Z).normalize();

        let projection_matrix =
            glam::Mat4::perspective_lh(self.fovy, self.aspect, self.z_near, self.z_far);

        // let view_matrix =
        //     glam::Mat4::look_at_lh(self.translation, self.translation + forward, self.up);

        projection_matrix
        // * view_matrix
    }

    // pub fn forward(&self) -> glam::Vec3 {
    //     let (x, _, z) = (self.rotation * glam::Vec3::Z).into();
    //     glam::Vec3::new(x, 0., z).normalize()
    // }

    // pub fn right(&self) -> glam::Vec3 {
    //     let (x, _, z) = (self.rotation * glam::Vec3::X).into();
    //     glam::Vec3::new(x, 0., z).normalize()
    // }

    // pub fn rotate_camera(&mut self, yaw: f32, pitch: f32) {
    //     let yaw_rotation = glam::Quat::from_rotation_y(yaw);
    //     let pitch_rotation = glam::Quat::from_rotation_x(pitch);

    //     self.rotation = yaw_rotation * self.rotation * pitch_rotation;
    // }
}

//====================================================================
