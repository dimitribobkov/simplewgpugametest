use crate::{Texture, Renderer, UniformUtils, Rc};
use wgpu::util::DeviceExt;

#[derive(std::fmt::Debug)]
pub struct Material{
    texture: Rc<Texture>,
    shininess: f32,
    metallic: f32,
}

impl Material{
    pub fn new(texture: Rc<Texture>, shininess: f32, metallic: f32) -> Self{
        Self{
            texture,
            shininess,
            metallic,

        }
    }

    pub fn borrow_texture(&self) -> &Rc<Texture>{
        &self.texture
    }

    fn create_uniform_layout(&self, renderer_reference: &Renderer) -> wgpu::BindGroupLayout{

        UniformUtils::create_bind_group_layout(renderer_reference, 0, wgpu::ShaderStage::FRAGMENT, Some("material"))
    }

    pub fn create_uniform_group(&mut self, renderer_reference: &Renderer) -> (wgpu::BindGroup, wgpu::BindGroupLayout, MaterialUniform, wgpu::Buffer){
        let material_uniform = MaterialUniform::new(self.shininess, self.metallic);
        let buffer = material_uniform.create_uniform_buffer(renderer_reference);
        let layout = self.create_uniform_layout(renderer_reference);
        (UniformUtils::create_bind_group(renderer_reference, &buffer, &layout, 0, Some("material")), layout, material_uniform, buffer)
    }

}


// We need this for Rust to store our data correctly for the shaders
#[repr(C)]
// This is so we can store this in a buffer
#[derive(Debug, Copy, Clone, bytemuck::Pod, bytemuck::Zeroable)]
pub struct MaterialUniform{
    shininess: f32,
    metallic: f32,
}
impl MaterialUniform{
    pub fn new(shininess: f32, metallic: f32) -> Self{
        Self{
            shininess,
            metallic
        }
    }
    pub fn create_uniform_buffer(&self, renderer_reference: &Renderer) -> wgpu::Buffer{
        renderer_reference.device.create_buffer_init(
            &wgpu::util::BufferInitDescriptor {
                label: Some("Material Uniform Buffer"),
                contents: bytemuck::cast_slice(&[*self]),
                usage: wgpu::BufferUsage::UNIFORM | wgpu::BufferUsage::COPY_DST,
            }
        )
    }

}
impl crate::UniformBuffer for MaterialUniform{

}