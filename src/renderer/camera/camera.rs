use cgmath::SquareMatrix;
use wgpu::util::DeviceExt;
use crate::{Renderer, UniformBuffer, UniformUtils};

pub struct Camera {
    pub eye: cgmath::Point3<f32>,
    pub target: cgmath::Point3<f32>,
    pub up: cgmath::Vector3<f32>,
    pub aspect: f32,
    pub fovy: f32,
    pub znear: f32,
    pub zfar: f32,
    pub width: u32,
    pub height: u32,
    buffer: wgpu::Buffer
}

#[rustfmt::skip]
pub const OPENGL_TO_WGPU_MATRIX: cgmath::Matrix4<f32> = cgmath::Matrix4::new(
    1.0, 0.0, 0.0, 0.0,
    0.0, 1.0, 0.0, 0.0,
    0.0, 0.0, 0.5, 0.0,
    0.0, 0.0, 0.5, 1.0,
);


impl Camera {
    pub fn new(renderer_reference: &Renderer, eye: cgmath::Point3<f32>, target: cgmath::Point3<f32>, up: cgmath::Vector3<f32>, aspect: f32, fovy: f32, znear: f32, zfar: f32) -> Self{
        Self{
            eye,
            target,
            up,
            aspect,
            fovy,
            znear,
            zfar,
            buffer: UniformUtils::generate_empty_buffer(renderer_reference),
            width: 1920,
            height: 1080,
        }
    }
    pub fn build_view_projection_matrix(&mut self, sc_desc: &wgpu::SwapChainDescriptor) -> (cgmath::Matrix4<f32>, cgmath::Matrix4<f32>) {
        // 1.
        let view = cgmath::Matrix4::look_at_rh(self.eye, self.target, self.up);
        // 2.
        let proj = cgmath::perspective(cgmath::Deg(self.fovy), self.aspect, self.znear, self.zfar);

        self.width = sc_desc.width;
        self.height = sc_desc.height;
        // 3.
        return (OPENGL_TO_WGPU_MATRIX * proj, view);
    }

    pub fn create_uniforms(&mut self, renderer_reference: &Renderer) -> (wgpu::BindGroup, wgpu::BindGroupLayout, CameraUniform){
        let mut translation_uniform = CameraUniform::new();
        let buffer = translation_uniform.create_uniform_buffer(renderer_reference);
        let layout = UniformUtils::create_bind_group_layout(renderer_reference, 0, wgpu::ShaderStage::VERTEX, Some("Transform"));
        self.buffer = buffer;
        (UniformUtils::create_bind_group(&renderer_reference, &self.buffer, &layout, 0, Some("Transform")), layout, translation_uniform)
    }

    pub fn get_buffer_reference(&self) -> &wgpu::Buffer{
        &self.buffer
    }

    pub fn move_camera(&mut self, mut new_pos: cgmath::Point3::<f32>){
        self.eye = lerp(cgmath::Point3::<f32> { x: self.target.x, y: self.target.y, z: new_pos.z }, new_pos, 0.075);
        new_pos.z = 0.0;
        self.target = lerp(self.target, new_pos, 0.075);
    }
}

 


// We need this for Rust to store our data correctly for the shaders
#[repr(C)]
// This is so we can store this in a buffer
#[derive(Debug, Copy, Clone, bytemuck::Pod, bytemuck::Zeroable)]
pub struct CameraUniform{
    pub proj: [[f32; 4]; 4],
    pub view: [[f32; 4]; 4],
}

impl CameraUniform{
    pub fn new() -> Self {
        Self {
            proj: cgmath::Matrix4::identity().into(),
            view: cgmath::Matrix4::identity().into(),
        }
    }

    pub fn update_view_proj(&mut self, camera: &mut Camera, sc_desc: &wgpu::SwapChainDescriptor) {
        let (proj, view) = camera.build_view_projection_matrix(sc_desc);
        self.proj = proj.into();
        self.view = view.into();

    }

    pub fn create_uniform_buffer(&self, renderer_reference:&Renderer) -> wgpu::Buffer{
        renderer_reference.device.create_buffer_init(
            &wgpu::util::BufferInitDescriptor {
                label: Some("Uniform Buffer"),
                contents: bytemuck::cast_slice(&[*self]),
                usage: wgpu::BufferUsage::UNIFORM | wgpu::BufferUsage::COPY_DST,
            }
        )
    }
}

impl UniformBuffer for CameraUniform{}
 
// Point3 lerping for camera smoothing
fn lerp(start: cgmath::Point3::<f32>, end: cgmath::Point3::<f32>, t: f32) -> cgmath::Point3::<f32>{
    cgmath::Point3::<f32> { x: start.x * (1.0 - t) + end.x * t, y: start.y * (1.0 - t) + end.y * t, z: start.z * (1.0 - t) + end.z * t}
}