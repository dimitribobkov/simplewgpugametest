use crate::{Vertex, RenderMesh, Entity, ComponentBase};
use std::any::Any;
use winit::{
    window::Window,
};

pub struct Renderer {
    pub surface: wgpu::Surface,
    pub device: wgpu::Device,
    pub queue: wgpu::Queue,
    pub sc_desc: wgpu::SwapChainDescriptor,
    swap_chain: wgpu::SwapChain,
    size: winit::dpi::PhysicalSize<u32>,
    pub render_pipeline: wgpu::RenderPipeline,
}

impl Renderer {
    // Creating some of the wgpu types requires async code
    pub async fn new(window: &Window) -> Self {
        let size = window.inner_size();

        // The instance is a handle to our GPU
        // BackendBit::PRIMARY => Vulkan + Metal + DX12 + Browser WebGPU
        let instance = wgpu::Instance::new(wgpu::BackendBit::DX12);
        let surface = unsafe { instance.create_surface(window) };
        
        let adapter = instance.request_adapter(
            &wgpu::RequestAdapterOptions {
                power_preference: wgpu::PowerPreference::HighPerformance,
                compatible_surface: Some(&surface),
            },
        ).await.unwrap();

        let (device, queue) = adapter.request_device(
            &wgpu::DeviceDescriptor {
                features: wgpu::Features::empty(),
                limits: wgpu::Limits::default(),
                shader_validation: true,
            },
            None, // Trace path
        ).await.unwrap();
        
        let sc_desc = wgpu::SwapChainDescriptor {
            usage: wgpu::TextureUsage::OUTPUT_ATTACHMENT,
            format: wgpu::TextureFormat::Bgra8UnormSrgb,
            width: size.width,
            height: size.height,
            present_mode: wgpu::PresentMode::Fifo,
        };

        let swap_chain = device.create_swap_chain(&surface, &sc_desc);

        

        // Create shader modules (Kind of like a link to the shaders). This links to a dummy shader (which will be changed after all uniform layouts have been gathered.)
        // The dummy shader also acts as a fallback shader
        let vs_module = device.create_shader_module(wgpu::include_spirv!("../shaders/dummy.vert.spv"));
        let fs_module = device.create_shader_module(wgpu::include_spirv!("../shaders/dummy.frag.spv"));
        let render_pipeline = Renderer::create_pipeline(&device, &sc_desc, vs_module, fs_module, &[]);



        Self {
            surface,
            device,
            queue,
            sc_desc,
            swap_chain,
            size,
            render_pipeline,
            
        }
    }

    
    fn create_pipeline(device: &wgpu::Device, sc_desc: &wgpu::SwapChainDescriptor, vs_module: wgpu::ShaderModule, fs_module: wgpu::ShaderModule,
        bind_group_layouts: &[&wgpu::BindGroupLayout]) -> wgpu::RenderPipeline {

       let render_pipeline_layout =
       device.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
           label: Some("Render Pipeline Layout"),
           bind_group_layouts: bind_group_layouts,
           push_constant_ranges: &[],
       });

        device.create_render_pipeline(&wgpu::RenderPipelineDescriptor {
        label: Some("Render Pipeline"),
        layout: Some(&render_pipeline_layout),
        vertex_stage: wgpu::ProgrammableStageDescriptor {
            module: &vs_module,
            entry_point: "main", // 1.
        },
        fragment_stage: Some(wgpu::ProgrammableStageDescriptor { // 2.
            module: &fs_module,
            entry_point: "main",
        }),
        rasterization_state: Some(
            wgpu::RasterizationStateDescriptor {
                front_face: wgpu::FrontFace::Ccw,
                cull_mode: wgpu::CullMode::None,
                depth_bias: 0,
                depth_bias_slope_scale: 0.0,
                depth_bias_clamp: 0.0,
                clamp_depth: false,
            }
        ),
            color_states: &[
                wgpu::ColorStateDescriptor {
                    format: sc_desc.format,
                    color_blend: wgpu::BlendDescriptor {
                        src_factor: wgpu::BlendFactor::SrcAlpha,
                        dst_factor: wgpu::BlendFactor::OneMinusSrcAlpha,
                        operation: wgpu::BlendOperation::Add
                    },
                    alpha_blend: wgpu::BlendDescriptor {
                        src_factor: wgpu::BlendFactor::One,
                        dst_factor: wgpu::BlendFactor::OneMinusSrcAlpha,
                        operation: wgpu::BlendOperation::Add
                    },
                    //color_blend: wgpu::BlendDescriptor::REPLACE,
                    //alpha_blend: wgpu::BlendDescriptor::REPLACE,
                    write_mask: wgpu::ColorWrite::ALL
                }
            ]   ,

        primitive_topology: wgpu::PrimitiveTopology::TriangleList, // 1.
        depth_stencil_state: None, // 2.
        vertex_state: wgpu::VertexStateDescriptor {
            index_format: wgpu::IndexFormat::Uint32,
            vertex_buffers: &[
                Vertex::desc(), // Set our vertex buffer description here (Description defines the things like texcoords and normals)
            ],
        },
        sample_count: 1, // 5.
        sample_mask: !0, // 6.
        alpha_to_coverage_enabled: false, // 7.
    })
   }

   pub fn recreate_pipeline(&mut self, bind_group_layouts: &[&wgpu::BindGroupLayout]){
    let vs_module = self.device.create_shader_module(wgpu::include_spirv!("../shaders/shader.vert.spv"));
    let fs_module = self.device.create_shader_module(wgpu::include_spirv!("../shaders/shader.frag.spv"));
    self.render_pipeline = Renderer::create_pipeline(&self.device, &self.sc_desc, vs_module, fs_module, bind_group_layouts);
   }

    pub fn resize(&mut self, new_size: winit::dpi::PhysicalSize<u32>) {
        self.size = new_size;
        self.sc_desc.width = new_size.width;
        self.sc_desc.height = new_size.height;
        self.swap_chain = self.device.create_swap_chain(&self.surface, &self.sc_desc);
    }

    pub fn update(&mut self) {
        // Not sure what to run here, maybe pipeline switching for multishader support?
    }

    pub fn render(&mut self, clear_color: wgpu::Color, entities: &Vec::<Entity>) -> Result<(), wgpu::SwapChainError> {
        let frame = self
        .swap_chain
        .get_current_frame()?
        .output;

        let mut encoder = self.device.create_command_encoder(&wgpu::CommandEncoderDescriptor {
            label: Some("Render Encoder"),
        });
        {
            let mut render_pass = encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
                color_attachments: &[
                    wgpu::RenderPassColorAttachmentDescriptor {
                        attachment: &frame.view,
                        resolve_target: None,
                        ops: wgpu::Operations {
                            load: wgpu::LoadOp::Clear(clear_color),
                            store: true,
                        }
                    }
                ],
                depth_stencil_attachment: None,
            });

            render_pass.set_pipeline(&self.render_pipeline); // 2.
            let mut meshes = Vec::<&RenderMesh>::new();
            for entity in entities.iter(){
                match entity.get_component::<RenderMesh>(RenderMesh::get_component_id()){
                    Ok(rm) => { meshes.push(rm) }
                    Err(e) => panic!("{:?}", e)
                }
            }
            for mesh in meshes.iter(){
                // 0 - texture count is reserved for textures
                render_pass.set_bind_group(0, &mesh.borrow_material().borrow_texture().get_texture_group(), &[]);
                let mut i: u32 = 1;
                for uniform in mesh.get_uniforms().iter(){
                    render_pass.set_bind_group(i, &uniform, &[]);
                    i += 1;
                }
                render_pass.set_vertex_buffer(0, mesh.get_vertex_buffer().slice(..));
                if mesh.get_num_indices() == 0{
                    render_pass.draw(0..mesh.get_num_vertices(), 0..1)
                }else{
                    render_pass.set_index_buffer(mesh.get_index_buffer().slice(..));
                    render_pass.draw_indexed(0..mesh.get_num_indices(), 0, 0..1);
                }

            }
        }
        
        
        // submit will accept anything that implements IntoIter
        self.queue.submit(std::iter::once(encoder.finish()));
        
        Ok(())    
    }

    pub fn get_window_size(&self) -> winit::dpi::PhysicalSize<u32>{
        self.size
    }

    pub fn write_buffer<T: bytemuck::Pod>(&self, buffer: &wgpu::Buffer, offset: u64, uniforms: &[T]){
        self.queue.write_buffer(buffer, offset, bytemuck::cast_slice(uniforms));
    }
}