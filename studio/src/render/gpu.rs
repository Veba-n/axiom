use bytemuck::{Pod, Zeroable};
use eframe::egui_wgpu::wgpu;

#[repr(C)]
#[derive(Copy, Clone, Debug, Pod, Zeroable)]
pub struct GpuVertex {
    pub position: [f32; 3],
    pub uv: [f32; 2],
    pub color: [f32; 4], // R, G, B, A (0.0 to 1.0)
}

impl GpuVertex {
    pub fn desc<'a>() -> wgpu::VertexBufferLayout<'a> {
        wgpu::VertexBufferLayout {
            array_stride: std::mem::size_of::<GpuVertex>() as wgpu::BufferAddress,
            step_mode: wgpu::VertexStepMode::Vertex,
            attributes: &[
                wgpu::VertexAttribute { // position
                    offset: 0,
                    shader_location: 0,
                    format: wgpu::VertexFormat::Float32x3,
                },
                wgpu::VertexAttribute { // uv
                    offset: std::mem::size_of::<[f32; 3]>() as wgpu::BufferAddress,
                    shader_location: 1,
                    format: wgpu::VertexFormat::Float32x2,
                },
                wgpu::VertexAttribute { // color
                    offset: (std::mem::size_of::<[f32; 3]>() + std::mem::size_of::<[f32; 2]>()) as wgpu::BufferAddress,
                    shader_location: 2,
                    format: wgpu::VertexFormat::Float32x4,
                },
            ],
        }
    }
}


#[repr(C)]
#[derive(Copy, Clone, Debug, Pod, Zeroable)]
pub struct CameraUniform {
    pub view_proj: [[f32; 4]; 4],
}

pub struct Custom3dCallback {
    pub solid_vertices: Vec<GpuVertex>,
    pub line_vertices: Vec<GpuVertex>,
    pub camera_matrix: [[f32; 4]; 4],
}

impl eframe::egui_wgpu::CallbackTrait for Custom3dCallback {
    fn prepare(
        &self,
        device: &wgpu::Device,
        queue: &wgpu::Queue,
        _screen_descriptor: &eframe::egui_wgpu::ScreenDescriptor,
        _egui_encoder: &mut wgpu::CommandEncoder,
        resources: &mut eframe::egui_wgpu::CallbackResources,
    ) -> Vec<wgpu::CommandBuffer> {
        use wgpu::util::DeviceExt;
        
        let solid_len = self.solid_vertices.len();
        let solid_bytes = bytemuck::cast_slice(&self.solid_vertices);
        
        let mut create_new_solid = true;
        if let Some(existing) = resources.get::<SolidBuffer>() {
            if existing.capacity >= solid_len {
                if solid_len > 0 { queue.write_buffer(&existing.buffer, 0, solid_bytes); }
                create_new_solid = false;
            }
        }
        if create_new_solid {
            let cap = solid_len.next_power_of_two().max(1024);
            let buf = device.create_buffer(&wgpu::BufferDescriptor {
                label: Some("Axiom Solid Buffer"),
                size: (cap * std::mem::size_of::<GpuVertex>()) as u64,
                usage: wgpu::BufferUsages::VERTEX | wgpu::BufferUsages::COPY_DST,
                mapped_at_creation: false,
            });
            if solid_len > 0 { queue.write_buffer(&buf, 0, solid_bytes); }
            resources.insert(SolidBuffer { buffer: buf, capacity: cap });
        }

        let line_len = self.line_vertices.len();
        let line_bytes = bytemuck::cast_slice(&self.line_vertices);
        
        let mut create_new_line = true;
        if let Some(existing) = resources.get::<LineBuffer>() {
            if existing.capacity >= line_len {
                if line_len > 0 { queue.write_buffer(&existing.buffer, 0, line_bytes); }
                create_new_line = false;
            }
        }
        if create_new_line {
            let cap = line_len.next_power_of_two().max(1024);
            let buf = device.create_buffer(&wgpu::BufferDescriptor {
                label: Some("Axiom Line Buffer"),
                size: (cap * std::mem::size_of::<GpuVertex>()) as u64,
                usage: wgpu::BufferUsages::VERTEX | wgpu::BufferUsages::COPY_DST,
                mapped_at_creation: false,
            });
            if line_len > 0 { queue.write_buffer(&buf, 0, line_bytes); }
            resources.insert(LineBuffer { buffer: buf, capacity: cap });
        }

        let camera_uniform = CameraUniform { view_proj: self.camera_matrix };
        let camera_uniform_arr = [camera_uniform];
        let camera_bytes = bytemuck::cast_slice(&camera_uniform_arr);
        
        let mut create_new_camera = true;
        if let Some(existing) = resources.get::<CameraBuffer>() {
            queue.write_buffer(&existing.buffer, 0, camera_bytes);
            create_new_camera = false;
        }
        if create_new_camera {
            let buf = device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
                label: Some("Axiom Camera Buffer"),
                contents: camera_bytes,
                usage: wgpu::BufferUsages::UNIFORM | wgpu::BufferUsages::COPY_DST,
            });
            
            if let Some(custom_pipeline) = resources.get::<Custom3dPipeline>() {
                let bind_group = device.create_bind_group(&wgpu::BindGroupDescriptor {
                    layout: &custom_pipeline.camera_bind_group_layout,
                    entries: &[wgpu::BindGroupEntry {
                        binding: 0,
                        resource: buf.as_entire_binding(),
                    }],
                    label: Some("camera_bind_group"),
                });
                resources.insert(CameraBindGroup { group: bind_group });
            }
            resources.insert(CameraBuffer { buffer: buf });
        }

        Vec::new()
    }

    fn paint<'a>(
        &'a self,
        _info: egui::PaintCallbackInfo,
        render_pass: &mut wgpu::RenderPass<'a>,
        resources: &'a eframe::egui_wgpu::CallbackResources,
    ) {
        if let Some(custom_pipeline) = resources.get::<Custom3dPipeline>() {
            if let Some(camera_bg) = resources.get::<CameraBindGroup>() {
                if let Some(solid) = resources.get::<SolidBuffer>() {
                    if !self.solid_vertices.is_empty() {
                        render_pass.set_pipeline(&custom_pipeline.solid_pipeline);
                        render_pass.set_bind_group(0, &camera_bg.group, &[]);
                        render_pass.set_vertex_buffer(0, solid.buffer.slice(..));
                        render_pass.draw(0..self.solid_vertices.len() as u32, 0..1);
                    }
                }
                if let Some(line) = resources.get::<LineBuffer>() {
                    if !self.line_vertices.is_empty() {
                        render_pass.set_pipeline(&custom_pipeline.line_pipeline);
                        render_pass.set_bind_group(0, &camera_bg.group, &[]);
                        render_pass.set_vertex_buffer(0, line.buffer.slice(..));
                        render_pass.draw(0..self.line_vertices.len() as u32, 0..1);
                    }
                }
            }
        }
    }
}

struct SolidBuffer { buffer: wgpu::Buffer, capacity: usize }
struct LineBuffer { buffer: wgpu::Buffer, capacity: usize }
struct CameraBuffer { buffer: wgpu::Buffer }
struct CameraBindGroup { group: wgpu::BindGroup }

pub struct Custom3dPipeline {
    pub solid_pipeline: wgpu::RenderPipeline,
    pub line_pipeline: wgpu::RenderPipeline,
    pub camera_bind_group_layout: wgpu::BindGroupLayout,
}

impl Custom3dPipeline {
    pub fn new(device: &wgpu::Device, format: wgpu::TextureFormat) -> Self {
        let shader = device.create_shader_module(wgpu::ShaderModuleDescriptor {
            label: Some("Axiom WGSL Shader"),
            source: wgpu::ShaderSource::Wgsl(include_str!("shaders/main.wgsl").into()),
        });

        let camera_bind_group_layout = device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
            entries: &[
                wgpu::BindGroupLayoutEntry {
                    binding: 0,
                    visibility: wgpu::ShaderStages::VERTEX,
                    ty: wgpu::BindingType::Buffer {
                        ty: wgpu::BufferBindingType::Uniform,
                        has_dynamic_offset: false,
                        min_binding_size: None,
                    },
                    count: None,
                }
            ],
            label: Some("camera_bind_group_layout"),
        });

        let pipeline_layout = device.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
            label: Some("Axiom Pipeline Layout"),
            bind_group_layouts: &[&camera_bind_group_layout],
            push_constant_ranges: &[],
        });

        // KATI MODEL PIPELINE (TriangleList)
        let solid_pipeline = device.create_render_pipeline(&wgpu::RenderPipelineDescriptor {
            label: Some("Axiom Solid Pipeline"),
            layout: Some(&pipeline_layout),
            vertex: wgpu::VertexState { module: &shader, entry_point: "vs_main", buffers: &[GpuVertex::desc()] },
            fragment: Some(wgpu::FragmentState {
                module: &shader, entry_point: "fs_main",
                targets: &[Some(wgpu::ColorTargetState { format, blend: Some(wgpu::BlendState::ALPHA_BLENDING), write_mask: wgpu::ColorWrites::ALL })],
            }),
            primitive: wgpu::PrimitiveState {
                topology: wgpu::PrimitiveTopology::TriangleList,
                strip_index_format: None, front_face: wgpu::FrontFace::Ccw,
                cull_mode: None, // Culling kapalı (Poligonlar ters çizilse bile görünsün)
                unclipped_depth: false, polygon_mode: wgpu::PolygonMode::Fill, conservative: false,
            },
            depth_stencil: Some(wgpu::DepthStencilState {
                format: wgpu::TextureFormat::Depth24Plus,
                depth_write_enabled: true, // DONANIM DERİNLİK TESTİ AKTİF!
                depth_compare: wgpu::CompareFunction::LessEqual, // Yakın pikseller uzaktakileri örter
                stencil: wgpu::StencilState::default(),
                bias: wgpu::DepthBiasState {
                    constant: 0, 
                    slope_scale: 0.0,
                    clamp: 0.0,
                },
            }),
            multisample: wgpu::MultisampleState { count: 1, mask: !0, alpha_to_coverage_enabled: false },
            multiview: None,
        });

        // ÇİZGİ/WİREFRAME PIPELINE (LineList)
        let line_pipeline = device.create_render_pipeline(&wgpu::RenderPipelineDescriptor {
            label: Some("Axiom Line Pipeline"),
            layout: Some(&pipeline_layout),
            vertex: wgpu::VertexState { module: &shader, entry_point: "vs_main", buffers: &[GpuVertex::desc()] },
            fragment: Some(wgpu::FragmentState {
                module: &shader, entry_point: "fs_main",
                targets: &[Some(wgpu::ColorTargetState { format, blend: Some(wgpu::BlendState::ALPHA_BLENDING), write_mask: wgpu::ColorWrites::ALL })],
            }),
            primitive: wgpu::PrimitiveState {
                topology: wgpu::PrimitiveTopology::LineList,
                strip_index_format: None, front_face: wgpu::FrontFace::Ccw,
                cull_mode: None,
                unclipped_depth: false, polygon_mode: wgpu::PolygonMode::Fill, conservative: false,
            },
            depth_stencil: Some(wgpu::DepthStencilState {
                format: wgpu::TextureFormat::Depth24Plus,
                depth_write_enabled: false, // Çizgiler depth yazmaz (solid'in arkasına girmez)
                depth_compare: wgpu::CompareFunction::LessEqual, // Ama solid'in arkasındaki çizgiler gizlenir!
                stencil: wgpu::StencilState::default(),
                bias: wgpu::DepthBiasState {
                    constant: -2, // Çizgileri hafifçe öne çek ki Z-fighting olmasın
                    slope_scale: -1.0,
                    clamp: 0.0,
                },
            }),
            multisample: wgpu::MultisampleState { count: 1, mask: !0, alpha_to_coverage_enabled: false },
            multiview: None,
        });

        Self { solid_pipeline, line_pipeline, camera_bind_group_layout }
    }
}
