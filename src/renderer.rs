use std::{borrow::Cow, sync::Arc};

use bytemuck::{Pod, Zeroable};
use eframe::{
    egui::{Rect, epaint::PaintCallbackInfo},
    egui_wgpu::{
        Callback, CallbackResources, CallbackTrait, ScreenDescriptor,
        wgpu::{self, util::DeviceExt},
    },
};
use glam::{Mat4, Vec3};

use crate::{
    BodyPart, BrushSize, Camera, Face, Layer, LayerVisibility, ModelArrangement, ModelHit,
    ModelKind, Skin, arranged_model_boxes, brush_footprint, face_region, model_boxes,
    skin::{SKIN_HEIGHT, SKIN_WIDTH},
};

// Egui keeps UI colors and its preferred framebuffer in gamma-encoded sRGB values. Keep the model
// intermediates in that same representation so model texels and color swatches compare exactly.
const MODEL_COLOR_FORMAT: wgpu::TextureFormat = wgpu::TextureFormat::Rgba8Unorm;

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct TextureUpdate {
    pub x: u8,
    pub y: u8,
    pub width: u8,
    pub height: u8,
}

impl TextureUpdate {
    pub const FULL: Self = Self {
        x: 0,
        y: 0,
        width: SKIN_WIDTH as u8,
        height: SKIN_HEIGHT as u8,
    };

    pub fn between(before: &Skin, after: &Skin) -> Option<Self> {
        let mut min_x = SKIN_WIDTH;
        let mut min_y = SKIN_HEIGHT;
        let mut max_x = 0;
        let mut max_y = 0;
        let mut changed = false;
        for y in 0..SKIN_HEIGHT {
            for x in 0..SKIN_WIDTH {
                let index = y * SKIN_WIDTH + x;
                if before.pixels()[index] != after.pixels()[index] {
                    changed = true;
                    min_x = min_x.min(x);
                    min_y = min_y.min(y);
                    max_x = max_x.max(x);
                    max_y = max_y.max(y);
                }
            }
        }
        if !changed {
            return None;
        }
        Some(Self {
            x: min_x as u8,
            y: min_y as u8,
            width: (max_x - min_x + 1) as u8,
            height: (max_y - min_y + 1) as u8,
        })
    }
}

#[derive(Clone)]
pub struct ModelPaintCallback {
    pub rect: Rect,
    pub kind: ModelKind,
    pub visibility: LayerVisibility,
    pub camera: Camera,
    pub skin: Arc<Skin>,
    pub texture_update: Option<TextureUpdate>,
    pub preview_hit: Option<ModelHit>,
    pub brush_size: BrushSize,
    pub solo_part: Option<BodyPart>,
    pub arrangement: ModelArrangement,
}

impl ModelPaintCallback {
    pub fn paint_callback(self) -> eframe::egui::PaintCallback {
        Callback::new_paint_callback(self.rect, self)
    }
}

impl CallbackTrait for ModelPaintCallback {
    fn prepare(
        &self,
        device: &wgpu::Device,
        queue: &wgpu::Queue,
        screen: &ScreenDescriptor,
        encoder: &mut wgpu::CommandEncoder,
        resources: &mut CallbackResources,
    ) -> Vec<wgpu::CommandBuffer> {
        let renderer: &mut ModelRenderer = resources
            .get_mut()
            .expect("model renderer was installed at app creation");
        renderer.prepare(device, queue, encoder, screen, self);
        Vec::new()
    }

    fn paint(
        &self,
        _info: PaintCallbackInfo,
        pass: &mut wgpu::RenderPass<'static>,
        resources: &CallbackResources,
    ) {
        let renderer: &ModelRenderer = resources
            .get()
            .expect("model renderer was installed at app creation");
        renderer.paint(pass);
    }
}

#[repr(C)]
#[derive(Clone, Copy, Pod, Zeroable)]
struct Vertex {
    position: [f32; 3],
    uv: [f32; 2],
}

impl Vertex {
    const LAYOUT: wgpu::VertexBufferLayout<'static> = wgpu::VertexBufferLayout {
        array_stride: size_of::<Self>() as u64,
        step_mode: wgpu::VertexStepMode::Vertex,
        attributes: &wgpu::vertex_attr_array![0 => Float32x3, 1 => Float32x2],
    };
}

#[repr(C)]
#[derive(Clone, Copy, Pod, Zeroable)]
struct PreviewVertex {
    position: [f32; 3],
}

#[repr(C)]
#[derive(Clone, Copy, Pod, Zeroable)]
struct GuideVertex {
    position: [f32; 3],
}

impl GuideVertex {
    const LAYOUT: wgpu::VertexBufferLayout<'static> = wgpu::VertexBufferLayout {
        array_stride: size_of::<Self>() as u64,
        step_mode: wgpu::VertexStepMode::Vertex,
        attributes: &wgpu::vertex_attr_array![0 => Float32x3],
    };
}

impl PreviewVertex {
    const LAYOUT: wgpu::VertexBufferLayout<'static> = wgpu::VertexBufferLayout {
        array_stride: size_of::<Self>() as u64,
        step_mode: wgpu::VertexStepMode::Vertex,
        attributes: &wgpu::vertex_attr_array![0 => Float32x3],
    };
}

#[repr(C)]
#[derive(Clone, Copy, Pod, Zeroable)]
struct SceneUniform {
    view_projection: [[f32; 4]; 4],
}

struct RenderTargets {
    _color: wgpu::Texture,
    color_view: wgpu::TextureView,
    _depth: wgpu::Texture,
    depth_view: wgpu::TextureView,
    composite_bind_group: wgpu::BindGroup,
    size: [u32; 2],
}

pub struct ModelRenderer {
    model_pipeline: wgpu::RenderPipeline,
    preview_pipeline: wgpu::RenderPipeline,
    guide_pipeline: wgpu::RenderPipeline,
    composite_pipeline: wgpu::RenderPipeline,
    skin_texture: wgpu::Texture,
    skin_bind_group: wgpu::BindGroup,
    scene_buffer: wgpu::Buffer,
    scene_bind_group: wgpu::BindGroup,
    vertex_buffer: wgpu::Buffer,
    vertex_capacity: usize,
    vertex_count: u32,
    preview_buffer: wgpu::Buffer,
    preview_vertex_count: u32,
    guide_buffer: wgpu::Buffer,
    guide_vertex_count: u32,
    composite_layout: wgpu::BindGroupLayout,
    sampler: wgpu::Sampler,
    targets: Option<RenderTargets>,
}

impl ModelRenderer {
    pub fn install(render_state: &eframe::egui_wgpu::RenderState, skin: &Skin) {
        let renderer = Self::new(
            &render_state.device,
            &render_state.queue,
            render_state.target_format,
            skin,
        );
        render_state
            .renderer
            .write()
            .callback_resources
            .insert(renderer);
    }

    fn new(
        device: &wgpu::Device,
        queue: &wgpu::Queue,
        surface_format: wgpu::TextureFormat,
        skin: &Skin,
    ) -> Self {
        let sampler = device.create_sampler(&wgpu::SamplerDescriptor {
            label: Some("skin nearest sampler"),
            mag_filter: wgpu::FilterMode::Nearest,
            min_filter: wgpu::FilterMode::Nearest,
            mipmap_filter: wgpu::MipmapFilterMode::Nearest,
            ..Default::default()
        });
        let skin_texture = device.create_texture(&wgpu::TextureDescriptor {
            label: Some("shared skin texture"),
            size: wgpu::Extent3d {
                width: SKIN_WIDTH as u32,
                height: SKIN_HEIGHT as u32,
                depth_or_array_layers: 1,
            },
            mip_level_count: 1,
            sample_count: 1,
            dimension: wgpu::TextureDimension::D2,
            format: MODEL_COLOR_FORMAT,
            usage: wgpu::TextureUsages::TEXTURE_BINDING | wgpu::TextureUsages::COPY_DST,
            view_formats: &[],
        });
        let skin_layout = device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
            label: Some("skin texture layout"),
            entries: &[
                wgpu::BindGroupLayoutEntry {
                    binding: 0,
                    visibility: wgpu::ShaderStages::FRAGMENT,
                    ty: wgpu::BindingType::Texture {
                        sample_type: wgpu::TextureSampleType::Float { filterable: true },
                        view_dimension: wgpu::TextureViewDimension::D2,
                        multisampled: false,
                    },
                    count: None,
                },
                wgpu::BindGroupLayoutEntry {
                    binding: 1,
                    visibility: wgpu::ShaderStages::FRAGMENT,
                    ty: wgpu::BindingType::Sampler(wgpu::SamplerBindingType::Filtering),
                    count: None,
                },
            ],
        });
        let skin_view = skin_texture.create_view(&Default::default());
        let skin_bind_group = device.create_bind_group(&wgpu::BindGroupDescriptor {
            label: Some("skin texture bind group"),
            layout: &skin_layout,
            entries: &[
                wgpu::BindGroupEntry {
                    binding: 0,
                    resource: wgpu::BindingResource::TextureView(&skin_view),
                },
                wgpu::BindGroupEntry {
                    binding: 1,
                    resource: wgpu::BindingResource::Sampler(&sampler),
                },
            ],
        });
        let scene_buffer = device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
            label: Some("model scene uniform"),
            contents: bytemuck::bytes_of(&SceneUniform {
                view_projection: Mat4::IDENTITY.to_cols_array_2d(),
            }),
            usage: wgpu::BufferUsages::UNIFORM | wgpu::BufferUsages::COPY_DST,
        });
        let scene_layout = device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
            label: Some("model scene layout"),
            entries: &[wgpu::BindGroupLayoutEntry {
                binding: 0,
                visibility: wgpu::ShaderStages::VERTEX,
                ty: wgpu::BindingType::Buffer {
                    ty: wgpu::BufferBindingType::Uniform,
                    has_dynamic_offset: false,
                    min_binding_size: None,
                },
                count: None,
            }],
        });
        let scene_bind_group = device.create_bind_group(&wgpu::BindGroupDescriptor {
            label: Some("model scene bind group"),
            layout: &scene_layout,
            entries: &[wgpu::BindGroupEntry {
                binding: 0,
                resource: scene_buffer.as_entire_binding(),
            }],
        });
        let model_shader = device.create_shader_module(wgpu::ShaderModuleDescriptor {
            label: Some("skin model shader"),
            source: wgpu::ShaderSource::Wgsl(Cow::Borrowed(MODEL_SHADER)),
        });
        let model_layout = device.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
            label: Some("skin model pipeline layout"),
            bind_group_layouts: &[Some(&scene_layout), Some(&skin_layout)],
            immediate_size: 0,
        });
        let model_pipeline = device.create_render_pipeline(&wgpu::RenderPipelineDescriptor {
            label: Some("skin model pipeline"),
            layout: Some(&model_layout),
            vertex: wgpu::VertexState {
                module: &model_shader,
                entry_point: Some("vertex_main"),
                buffers: &[Vertex::LAYOUT],
                compilation_options: Default::default(),
            },
            fragment: Some(wgpu::FragmentState {
                module: &model_shader,
                entry_point: Some("fragment_main"),
                targets: &[Some(wgpu::ColorTargetState {
                    format: MODEL_COLOR_FORMAT,
                    blend: Some(wgpu::BlendState::ALPHA_BLENDING),
                    write_mask: wgpu::ColorWrites::ALL,
                })],
                compilation_options: Default::default(),
            }),
            primitive: wgpu::PrimitiveState {
                cull_mode: None,
                ..Default::default()
            },
            depth_stencil: Some(wgpu::DepthStencilState {
                format: wgpu::TextureFormat::Depth32Float,
                depth_write_enabled: Some(true),
                depth_compare: Some(wgpu::CompareFunction::Less),
                stencil: Default::default(),
                bias: Default::default(),
            }),
            multisample: Default::default(),
            multiview_mask: None,
            cache: None,
        });

        let preview_shader = device.create_shader_module(wgpu::ShaderModuleDescriptor {
            label: Some("brush preview shader"),
            source: wgpu::ShaderSource::Wgsl(Cow::Borrowed(PREVIEW_SHADER)),
        });
        let preview_layout = device.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
            label: Some("brush preview pipeline layout"),
            bind_group_layouts: &[Some(&scene_layout)],
            immediate_size: 0,
        });
        let preview_pipeline = device.create_render_pipeline(&wgpu::RenderPipelineDescriptor {
            label: Some("brush preview pipeline"),
            layout: Some(&preview_layout),
            vertex: wgpu::VertexState {
                module: &preview_shader,
                entry_point: Some("vertex_main"),
                buffers: &[PreviewVertex::LAYOUT],
                compilation_options: Default::default(),
            },
            fragment: Some(wgpu::FragmentState {
                module: &preview_shader,
                entry_point: Some("fragment_main"),
                targets: &[Some(wgpu::ColorTargetState {
                    format: MODEL_COLOR_FORMAT,
                    blend: Some(wgpu::BlendState::ALPHA_BLENDING),
                    write_mask: wgpu::ColorWrites::ALL,
                })],
                compilation_options: Default::default(),
            }),
            primitive: wgpu::PrimitiveState {
                cull_mode: None,
                ..Default::default()
            },
            depth_stencil: Some(wgpu::DepthStencilState {
                format: wgpu::TextureFormat::Depth32Float,
                depth_write_enabled: Some(false),
                depth_compare: Some(wgpu::CompareFunction::LessEqual),
                stencil: Default::default(),
                bias: Default::default(),
            }),
            multisample: Default::default(),
            multiview_mask: None,
            cache: None,
        });
        let guide_shader = device.create_shader_module(wgpu::ShaderModuleDescriptor {
            label: Some("solo guide shader"),
            source: wgpu::ShaderSource::Wgsl(Cow::Borrowed(GUIDE_SHADER)),
        });
        let guide_layout = device.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
            label: Some("solo guide pipeline layout"),
            bind_group_layouts: &[Some(&scene_layout)],
            immediate_size: 0,
        });
        let guide_pipeline = device.create_render_pipeline(&wgpu::RenderPipelineDescriptor {
            label: Some("solo guide pipeline"),
            layout: Some(&guide_layout),
            vertex: wgpu::VertexState {
                module: &guide_shader,
                entry_point: Some("vertex_main"),
                buffers: &[GuideVertex::LAYOUT],
                compilation_options: Default::default(),
            },
            fragment: Some(wgpu::FragmentState {
                module: &guide_shader,
                entry_point: Some("fragment_main"),
                targets: &[Some(wgpu::ColorTargetState {
                    format: MODEL_COLOR_FORMAT,
                    blend: Some(wgpu::BlendState::ALPHA_BLENDING),
                    write_mask: wgpu::ColorWrites::ALL,
                })],
                compilation_options: Default::default(),
            }),
            primitive: wgpu::PrimitiveState {
                topology: wgpu::PrimitiveTopology::LineList,
                cull_mode: None,
                ..Default::default()
            },
            depth_stencil: Some(wgpu::DepthStencilState {
                format: wgpu::TextureFormat::Depth32Float,
                depth_write_enabled: Some(false),
                depth_compare: Some(wgpu::CompareFunction::Always),
                stencil: Default::default(),
                bias: Default::default(),
            }),
            multisample: Default::default(),
            multiview_mask: None,
            cache: None,
        });

        let composite_layout = device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
            label: Some("model composite layout"),
            entries: &[
                wgpu::BindGroupLayoutEntry {
                    binding: 0,
                    visibility: wgpu::ShaderStages::FRAGMENT,
                    ty: wgpu::BindingType::Texture {
                        sample_type: wgpu::TextureSampleType::Float { filterable: true },
                        view_dimension: wgpu::TextureViewDimension::D2,
                        multisampled: false,
                    },
                    count: None,
                },
                wgpu::BindGroupLayoutEntry {
                    binding: 1,
                    visibility: wgpu::ShaderStages::FRAGMENT,
                    ty: wgpu::BindingType::Sampler(wgpu::SamplerBindingType::Filtering),
                    count: None,
                },
            ],
        });
        let composite_shader = device.create_shader_module(wgpu::ShaderModuleDescriptor {
            label: Some("model composite shader"),
            source: wgpu::ShaderSource::Wgsl(Cow::Borrowed(COMPOSITE_SHADER)),
        });
        let composite_pipeline_layout =
            device.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
                label: Some("model composite pipeline layout"),
                bind_group_layouts: &[Some(&composite_layout)],
                immediate_size: 0,
            });
        let composite_pipeline = device.create_render_pipeline(&wgpu::RenderPipelineDescriptor {
            label: Some("model composite pipeline"),
            layout: Some(&composite_pipeline_layout),
            vertex: wgpu::VertexState {
                module: &composite_shader,
                entry_point: Some("vertex_main"),
                buffers: &[],
                compilation_options: Default::default(),
            },
            fragment: Some(wgpu::FragmentState {
                module: &composite_shader,
                entry_point: Some(composite_fragment_entry(surface_format)),
                targets: &[Some(wgpu::ColorTargetState {
                    format: surface_format,
                    blend: Some(wgpu::BlendState::REPLACE),
                    write_mask: wgpu::ColorWrites::ALL,
                })],
                compilation_options: Default::default(),
            }),
            primitive: Default::default(),
            depth_stencil: None,
            multisample: Default::default(),
            multiview_mask: None,
            cache: None,
        });
        let vertex_capacity = 12 * 6 * 6;
        let vertex_buffer = device.create_buffer(&wgpu::BufferDescriptor {
            label: Some("skin model vertices"),
            size: (vertex_capacity * size_of::<Vertex>()) as u64,
            usage: wgpu::BufferUsages::VERTEX | wgpu::BufferUsages::COPY_DST,
            mapped_at_creation: false,
        });
        let preview_buffer = device.create_buffer(&wgpu::BufferDescriptor {
            label: Some("brush preview vertices"),
            size: (16 * 6 * size_of::<PreviewVertex>()) as u64,
            usage: wgpu::BufferUsages::VERTEX | wgpu::BufferUsages::COPY_DST,
            mapped_at_creation: false,
        });
        let guide_buffer = device.create_buffer(&wgpu::BufferDescriptor {
            label: Some("solo guide vertices"),
            size: (BodyPart::ALL.len() * 12 * 2 * size_of::<GuideVertex>()) as u64,
            usage: wgpu::BufferUsages::VERTEX | wgpu::BufferUsages::COPY_DST,
            mapped_at_creation: false,
        });
        let mut renderer = Self {
            model_pipeline,
            preview_pipeline,
            guide_pipeline,
            composite_pipeline,
            skin_texture,
            skin_bind_group,
            scene_buffer,
            scene_bind_group,
            vertex_buffer,
            vertex_capacity,
            vertex_count: 0,
            preview_buffer,
            preview_vertex_count: 0,
            guide_buffer,
            guide_vertex_count: 0,
            composite_layout,
            sampler,
            targets: None,
        };
        renderer.upload_texture(queue, skin, TextureUpdate::FULL);
        renderer
    }

    fn prepare(
        &mut self,
        device: &wgpu::Device,
        queue: &wgpu::Queue,
        encoder: &mut wgpu::CommandEncoder,
        screen: &ScreenDescriptor,
        callback: &ModelPaintCallback,
    ) {
        if let Some(update) = callback.texture_update {
            self.upload_texture(queue, &callback.skin, update);
        }
        let width = (callback.rect.width() * screen.pixels_per_point)
            .round()
            .max(1.0) as u32;
        let height = (callback.rect.height() * screen.pixels_per_point)
            .round()
            .max(1.0) as u32;
        self.ensure_targets(device, [width, height]);

        let vertices = model_vertices(
            callback.kind,
            callback.visibility,
            callback.solo_part,
            callback.arrangement,
        );
        debug_assert!(vertices.len() <= self.vertex_capacity);
        queue.write_buffer(&self.vertex_buffer, 0, bytemuck::cast_slice(&vertices));
        self.vertex_count = vertices.len() as u32;
        let preview_vertices = callback.preview_hit.map_or_else(Vec::new, |hit| {
            preview_vertices(
                callback.kind,
                hit,
                callback.brush_size,
                callback.arrangement,
            )
        });
        queue.write_buffer(
            &self.preview_buffer,
            0,
            bytemuck::cast_slice(&preview_vertices),
        );
        self.preview_vertex_count = preview_vertices.len() as u32;
        let guide_vertices = callback
            .solo_part
            .map_or_else(Vec::new, |part| guide_vertices(callback.kind, part));
        queue.write_buffer(&self.guide_buffer, 0, bytemuck::cast_slice(&guide_vertices));
        self.guide_vertex_count = guide_vertices.len() as u32;
        let aspect = width as f32 / height as f32;
        let half_height = callback.camera.orthographic_height * 0.5;
        let half_width = half_height * aspect;
        #[allow(deprecated)]
        let view = Mat4::look_at_rh(
            callback.camera.position(),
            callback.camera.target,
            camera_up(callback.camera),
        );
        #[allow(deprecated)]
        let projection = Mat4::orthographic_rh(
            -half_width,
            half_width,
            -half_height,
            half_height,
            0.1,
            callback.camera.distance * 2.0,
        );
        queue.write_buffer(
            &self.scene_buffer,
            0,
            bytemuck::bytes_of(&SceneUniform {
                view_projection: (projection * view).to_cols_array_2d(),
            }),
        );

        let targets = self.targets.as_ref().expect("render targets exist");
        let mut pass = encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
            label: Some("skin model offscreen pass"),
            color_attachments: &[Some(wgpu::RenderPassColorAttachment {
                view: &targets.color_view,
                resolve_target: None,
                depth_slice: None,
                ops: wgpu::Operations {
                    load: wgpu::LoadOp::Clear(wgpu::Color {
                        r: 0.055,
                        g: 0.065,
                        b: 0.08,
                        a: 1.0,
                    }),
                    store: wgpu::StoreOp::Store,
                },
            })],
            depth_stencil_attachment: Some(wgpu::RenderPassDepthStencilAttachment {
                view: &targets.depth_view,
                depth_ops: Some(wgpu::Operations {
                    load: wgpu::LoadOp::Clear(1.0),
                    store: wgpu::StoreOp::Store,
                }),
                stencil_ops: None,
            }),
            timestamp_writes: None,
            occlusion_query_set: None,
            multiview_mask: None,
        });
        if self.guide_vertex_count > 0 {
            pass.set_pipeline(&self.guide_pipeline);
            pass.set_bind_group(0, &self.scene_bind_group, &[]);
            pass.set_vertex_buffer(0, self.guide_buffer.slice(..));
            pass.draw(0..self.guide_vertex_count, 0..1);
        }
        pass.set_pipeline(&self.model_pipeline);
        pass.set_bind_group(0, &self.scene_bind_group, &[]);
        pass.set_bind_group(1, &self.skin_bind_group, &[]);
        pass.set_vertex_buffer(0, self.vertex_buffer.slice(..));
        pass.draw(0..self.vertex_count, 0..1);
        if self.preview_vertex_count > 0 {
            pass.set_pipeline(&self.preview_pipeline);
            pass.set_bind_group(0, &self.scene_bind_group, &[]);
            pass.set_vertex_buffer(0, self.preview_buffer.slice(..));
            pass.draw(0..self.preview_vertex_count, 0..1);
        }
    }

    fn paint(&self, pass: &mut wgpu::RenderPass<'static>) {
        let targets = self.targets.as_ref().expect("render targets exist");
        pass.set_pipeline(&self.composite_pipeline);
        pass.set_bind_group(0, &targets.composite_bind_group, &[]);
        pass.draw(0..3, 0..1);
    }

    fn ensure_targets(&mut self, device: &wgpu::Device, size: [u32; 2]) {
        if self
            .targets
            .as_ref()
            .is_some_and(|targets| targets.size == size)
        {
            return;
        }
        let color = device.create_texture(&wgpu::TextureDescriptor {
            label: Some("model offscreen color"),
            size: wgpu::Extent3d {
                width: size[0],
                height: size[1],
                depth_or_array_layers: 1,
            },
            mip_level_count: 1,
            sample_count: 1,
            dimension: wgpu::TextureDimension::D2,
            format: MODEL_COLOR_FORMAT,
            usage: wgpu::TextureUsages::RENDER_ATTACHMENT | wgpu::TextureUsages::TEXTURE_BINDING,
            view_formats: &[],
        });
        let color_view = color.create_view(&Default::default());
        let depth = device.create_texture(&wgpu::TextureDescriptor {
            label: Some("model depth buffer"),
            size: wgpu::Extent3d {
                width: size[0],
                height: size[1],
                depth_or_array_layers: 1,
            },
            mip_level_count: 1,
            sample_count: 1,
            dimension: wgpu::TextureDimension::D2,
            format: wgpu::TextureFormat::Depth32Float,
            usage: wgpu::TextureUsages::RENDER_ATTACHMENT,
            view_formats: &[],
        });
        let depth_view = depth.create_view(&Default::default());
        let composite_bind_group = device.create_bind_group(&wgpu::BindGroupDescriptor {
            label: Some("model composite bind group"),
            layout: &self.composite_layout,
            entries: &[
                wgpu::BindGroupEntry {
                    binding: 0,
                    resource: wgpu::BindingResource::TextureView(&color_view),
                },
                wgpu::BindGroupEntry {
                    binding: 1,
                    resource: wgpu::BindingResource::Sampler(&self.sampler),
                },
            ],
        });
        self.targets = Some(RenderTargets {
            _color: color,
            color_view,
            _depth: depth,
            depth_view,
            composite_bind_group,
            size,
        });
    }

    fn upload_texture(&mut self, queue: &wgpu::Queue, skin: &Skin, update: TextureUpdate) {
        let mut bytes =
            Vec::with_capacity(usize::from(update.width) * usize::from(update.height) * 4);
        for y in update.y..update.y + update.height {
            let start = usize::from(y) * SKIN_WIDTH + usize::from(update.x);
            let end = start + usize::from(update.width);
            bytes.extend_from_slice(bytemuck::cast_slice(&skin.pixels()[start..end]));
        }
        queue.write_texture(
            wgpu::TexelCopyTextureInfo {
                texture: &self.skin_texture,
                mip_level: 0,
                origin: wgpu::Origin3d {
                    x: u32::from(update.x),
                    y: u32::from(update.y),
                    z: 0,
                },
                aspect: wgpu::TextureAspect::All,
            },
            &bytes,
            wgpu::TexelCopyBufferLayout {
                offset: 0,
                bytes_per_row: Some(u32::from(update.width) * 4),
                rows_per_image: Some(u32::from(update.height)),
            },
            wgpu::Extent3d {
                width: u32::from(update.width),
                height: u32::from(update.height),
                depth_or_array_layers: 1,
            },
        );
    }
}

fn camera_up(camera: Camera) -> Vec3 {
    let right = camera.view_direction().cross(Vec3::Y).normalize_or_zero();
    right.cross(camera.view_direction()).normalize_or_zero()
}

fn composite_fragment_entry(surface_format: wgpu::TextureFormat) -> &'static str {
    // An sRGB render target expects linear shader output and applies the transfer function on
    // write. Egui normally chooses a non-sRGB target, which expects gamma values directly.
    if surface_format.is_srgb() {
        "fragment_linear"
    } else {
        "fragment_gamma"
    }
}

fn model_vertices(
    kind: ModelKind,
    visibility: LayerVisibility,
    part: Option<BodyPart>,
    arrangement: ModelArrangement,
) -> Vec<Vertex> {
    let mut vertices = Vec::with_capacity(12 * 6 * 6);
    for layer in [Layer::Base, Layer::Outer] {
        let visible = match layer {
            Layer::Base => visibility.base,
            Layer::Outer => visibility.outer,
        };
        if !visible {
            continue;
        }
        for model_box in arranged_model_boxes(kind, arrangement)
            .into_iter()
            .filter(|item| item.layer == layer && part.is_none_or(|part| item.part == part))
        {
            for face in Face::ALL {
                let corners = face_corners(model_box.min, model_box.max, face);
                let region = face_region(kind, model_box.part, layer, face);
                let x0 = f32::from(region.rect.x) / SKIN_WIDTH as f32;
                let x1 = f32::from(region.rect.x + region.rect.width) / SKIN_WIDTH as f32;
                let y0 = f32::from(region.rect.y) / SKIN_HEIGHT as f32;
                let y1 = f32::from(region.rect.y + region.rect.height) / SKIN_HEIGHT as f32;
                let (u0, u1) = if region.flip_u { (x1, x0) } else { (x0, x1) };
                let (v0, v1) = if region.flip_v { (y1, y0) } else { (y0, y1) };
                let quad = [
                    vertex(corners[0], [u0, v0]),
                    vertex(corners[1], [u1, v0]),
                    vertex(corners[2], [u1, v1]),
                    vertex(corners[0], [u0, v0]),
                    vertex(corners[2], [u1, v1]),
                    vertex(corners[3], [u0, v1]),
                ];
                vertices.extend_from_slice(&quad);
            }
        }
    }
    vertices
}

fn guide_vertices(kind: ModelKind, solo_part: BodyPart) -> Vec<GuideVertex> {
    const EDGES: [(usize, usize); 12] = [
        (0, 1),
        (1, 5),
        (5, 4),
        (4, 0),
        (2, 3),
        (3, 7),
        (7, 6),
        (6, 2),
        (0, 2),
        (1, 3),
        (5, 7),
        (4, 6),
    ];
    let mut vertices = Vec::with_capacity((BodyPart::ALL.len() - 1) * EDGES.len() * 2);
    for model_box in model_boxes(kind)
        .into_iter()
        .filter(|item| item.layer == Layer::Base && item.part != solo_part)
    {
        let min = model_box.min;
        let max = model_box.max;
        let corners = [
            Vec3::new(min.x, min.y, min.z),
            Vec3::new(max.x, min.y, min.z),
            Vec3::new(min.x, max.y, min.z),
            Vec3::new(max.x, max.y, min.z),
            Vec3::new(min.x, min.y, max.z),
            Vec3::new(max.x, min.y, max.z),
            Vec3::new(min.x, max.y, max.z),
            Vec3::new(max.x, max.y, max.z),
        ];
        for (start, end) in EDGES {
            vertices.push(GuideVertex {
                position: corners[start].to_array(),
            });
            vertices.push(GuideVertex {
                position: corners[end].to_array(),
            });
        }
    }
    vertices
}

fn vertex(position: Vec3, uv: [f32; 2]) -> Vertex {
    Vertex {
        position: position.to_array(),
        uv,
    }
}

fn preview_vertices(
    kind: ModelKind,
    hit: ModelHit,
    size: BrushSize,
    arrangement: ModelArrangement,
) -> Vec<PreviewVertex> {
    let Some(model_box) = arranged_model_boxes(kind, arrangement)
        .into_iter()
        .find(|item| item.part == hit.part && item.layer == hit.layer)
    else {
        return Vec::new();
    };
    let region = face_region(kind, hit.part, hit.layer, hit.face);
    let corners = face_corners(model_box.min, model_box.max, hit.face);
    let normal = face_normal(hit.face) * 0.01;
    let mut vertices = Vec::with_capacity(usize::from(size.pixels()).pow(2) * 6);
    for texel in brush_footprint(kind, hit, size) {
        let atlas_u = texel.x - region.rect.x;
        let atlas_v = texel.y - region.rect.y;
        let local_u = if region.flip_u {
            region.rect.width - 1 - atlas_u
        } else {
            atlas_u
        };
        let local_v = if region.flip_v {
            region.rect.height - 1 - atlas_v
        } else {
            atlas_v
        };
        let u0 = f32::from(local_u) / f32::from(region.rect.width);
        let u1 = f32::from(local_u + 1) / f32::from(region.rect.width);
        let v0 = f32::from(local_v) / f32::from(region.rect.height);
        let v1 = f32::from(local_v + 1) / f32::from(region.rect.height);
        let quad = [
            face_point(corners, u0, v0) + normal,
            face_point(corners, u1, v0) + normal,
            face_point(corners, u1, v1) + normal,
            face_point(corners, u0, v1) + normal,
        ];
        vertices.extend([
            PreviewVertex {
                position: quad[0].to_array(),
            },
            PreviewVertex {
                position: quad[1].to_array(),
            },
            PreviewVertex {
                position: quad[2].to_array(),
            },
            PreviewVertex {
                position: quad[0].to_array(),
            },
            PreviewVertex {
                position: quad[2].to_array(),
            },
            PreviewVertex {
                position: quad[3].to_array(),
            },
        ]);
    }
    vertices
}

fn face_point(corners: [Vec3; 4], u: f32, v: f32) -> Vec3 {
    corners[0]
        .lerp(corners[1], u)
        .lerp(corners[3].lerp(corners[2], u), v)
}

fn face_normal(face: Face) -> Vec3 {
    match face {
        Face::Front => Vec3::Z,
        Face::Back => -Vec3::Z,
        Face::Left => Vec3::X,
        Face::Right => -Vec3::X,
        Face::Top => Vec3::Y,
        Face::Bottom => -Vec3::Y,
    }
}

fn face_corners(min: Vec3, max: Vec3, face: Face) -> [Vec3; 4] {
    match face {
        Face::Front => [
            Vec3::new(min.x, max.y, max.z),
            Vec3::new(max.x, max.y, max.z),
            Vec3::new(max.x, min.y, max.z),
            Vec3::new(min.x, min.y, max.z),
        ],
        Face::Back => [
            Vec3::new(min.x, max.y, min.z),
            Vec3::new(max.x, max.y, min.z),
            Vec3::new(max.x, min.y, min.z),
            Vec3::new(min.x, min.y, min.z),
        ],
        Face::Left => [
            Vec3::new(max.x, max.y, min.z),
            Vec3::new(max.x, max.y, max.z),
            Vec3::new(max.x, min.y, max.z),
            Vec3::new(max.x, min.y, min.z),
        ],
        Face::Right => [
            Vec3::new(min.x, max.y, min.z),
            Vec3::new(min.x, max.y, max.z),
            Vec3::new(min.x, min.y, max.z),
            Vec3::new(min.x, min.y, min.z),
        ],
        Face::Top => [
            Vec3::new(min.x, max.y, min.z),
            Vec3::new(max.x, max.y, min.z),
            Vec3::new(max.x, max.y, max.z),
            Vec3::new(min.x, max.y, max.z),
        ],
        Face::Bottom => [
            Vec3::new(min.x, min.y, min.z),
            Vec3::new(max.x, min.y, min.z),
            Vec3::new(max.x, min.y, max.z),
            Vec3::new(min.x, min.y, max.z),
        ],
    }
}

const MODEL_SHADER: &str = r#"
struct Scene {
    view_projection: mat4x4<f32>,
};

@group(0) @binding(0) var<uniform> scene: Scene;
@group(1) @binding(0) var skin: texture_2d<f32>;
@group(1) @binding(1) var skin_sampler: sampler;

struct VertexInput {
    @location(0) position: vec3<f32>,
    @location(1) uv: vec2<f32>,
};

struct VertexOutput {
    @builtin(position) position: vec4<f32>,
    @location(0) uv: vec2<f32>,
};

@vertex
fn vertex_main(input: VertexInput) -> VertexOutput {
    var output: VertexOutput;
    output.position = scene.view_projection * vec4<f32>(input.position, 1.0);
    output.uv = input.uv;
    return output;
}

@fragment
fn fragment_main(input: VertexOutput) -> @location(0) vec4<f32> {
    var color = textureSample(skin, skin_sampler, input.uv);
    if color.a < 0.004 {
        discard;
    }
    return color;
}
"#;

const PREVIEW_SHADER: &str = r#"
struct Scene {
    view_projection: mat4x4<f32>,
};

@group(0) @binding(0) var<uniform> scene: Scene;

@vertex
fn vertex_main(@location(0) position: vec3<f32>) -> @builtin(position) vec4<f32> {
    return scene.view_projection * vec4<f32>(position, 1.0);
}

@fragment
fn fragment_main() -> @location(0) vec4<f32> {
    return vec4<f32>(1.0, 0.68, 0.08, 0.58);
}
"#;

const GUIDE_SHADER: &str = r#"
struct Scene {
    view_projection: mat4x4<f32>,
};

@group(0) @binding(0) var<uniform> scene: Scene;

@vertex
fn vertex_main(@location(0) position: vec3<f32>) -> @builtin(position) vec4<f32> {
    return scene.view_projection * vec4<f32>(position, 1.0);
}

@fragment
fn fragment_main() -> @location(0) vec4<f32> {
    return vec4<f32>(0.72, 0.78, 0.88, 0.16);
}
"#;

const COMPOSITE_SHADER: &str = r#"
@group(0) @binding(0) var source: texture_2d<f32>;
@group(0) @binding(1) var source_sampler: sampler;

struct VertexOutput {
    @builtin(position) position: vec4<f32>,
    @location(0) uv: vec2<f32>,
};

@vertex
fn vertex_main(@builtin(vertex_index) index: u32) -> VertexOutput {
    var positions = array<vec2<f32>, 3>(
        vec2<f32>(-1.0, -1.0),
        vec2<f32>(3.0, -1.0),
        vec2<f32>(-1.0, 3.0)
    );
    var output: VertexOutput;
    output.position = vec4<f32>(positions[index], 0.0, 1.0);
    output.uv = vec2<f32>(
        (output.position.x + 1.0) * 0.5,
        (1.0 - output.position.y) * 0.5
    );
    return output;
}

fn linear_from_gamma_rgb(srgb: vec3<f32>) -> vec3<f32> {
    let cutoff = srgb < vec3<f32>(0.04045);
    let lower = srgb / vec3<f32>(12.92);
    let higher = pow((srgb + vec3<f32>(0.055)) / vec3<f32>(1.055), vec3<f32>(2.4));
    return select(higher, lower, cutoff);
}

@fragment
fn fragment_gamma(input: VertexOutput) -> @location(0) vec4<f32> {
    return textureSample(source, source_sampler, input.uv);
}

@fragment
fn fragment_linear(input: VertexOutput) -> @location(0) vec4<f32> {
    let color_gamma = textureSample(source, source_sampler, input.uv);
    return vec4<f32>(linear_from_gamma_rgb(color_gamma.rgb), color_gamma.a);
}
"#;

#[cfg(test)]
mod tests {
    use super::*;
    use crate::Texel;

    fn linear_from_gamma(value: f32) -> f32 {
        if value <= 0.04045 {
            value / 12.92
        } else {
            ((value + 0.055) / 1.055).powf(2.4)
        }
    }

    fn gamma_from_linear(value: f32) -> f32 {
        if value <= 0.003_130_8 {
            value * 12.92
        } else {
            1.055 * value.powf(1.0 / 2.4) - 0.055
        }
    }

    #[test]
    fn texture_update_is_minimal_bounding_rectangle() {
        let before = Skin::transparent();
        let mut document = crate::SkinDocument::from_skin(before.clone(), None);
        let mut stroke = crate::StrokeBuilder::new();
        for texel in [Texel::new(8, 8), Texel::new(12, 12)] {
            document.paint(
                &mut stroke,
                ModelKind::Classic,
                ModelHit {
                    part: crate::BodyPart::Head,
                    layer: Layer::Base,
                    face: Face::Front,
                    distance: 1.0,
                    texel,
                },
                crate::BrushSize::One,
                [1, 2, 3, 4],
            );
        }
        assert_eq!(
            TextureUpdate::between(&before, document.skin()),
            Some(TextureUpdate {
                x: 8,
                y: 8,
                width: 5,
                height: 5
            })
        );
        assert_eq!(TextureUpdate::between(&before, &before), None);
    }

    #[test]
    fn visibility_and_hit_control_generated_geometry() {
        let base = model_vertices(
            ModelKind::Classic,
            LayerVisibility::BASE_ONLY,
            None,
            ModelArrangement::Joined,
        );
        let all = model_vertices(
            ModelKind::Classic,
            LayerVisibility::ALL,
            None,
            ModelArrangement::Joined,
        );
        assert_eq!(base.len(), 6 * 6 * 6);
        assert_eq!(all.len(), 12 * 6 * 6);
        let solo = model_vertices(
            ModelKind::Classic,
            LayerVisibility::ALL,
            Some(BodyPart::RightArm),
            ModelArrangement::Joined,
        );
        assert_eq!(solo.len(), 2 * 6 * 6);

        let exploded = model_vertices(
            ModelKind::Classic,
            LayerVisibility::ALL,
            None,
            ModelArrangement::Exploded,
        );
        assert_eq!(exploded.len(), all.len());
        assert_ne!(exploded[0].position, all[0].position);
    }

    #[test]
    fn model_color_intermediates_preserve_gamma_encoded_skin_bytes() {
        assert_eq!(MODEL_COLOR_FORMAT, wgpu::TextureFormat::Rgba8Unorm);
        assert_eq!(size_of::<Vertex>(), 5 * size_of::<f32>());
    }

    #[test]
    fn composite_transfer_matches_egui_for_gamma_and_srgb_framebuffers() {
        for format in [
            wgpu::TextureFormat::Rgba8Unorm,
            wgpu::TextureFormat::Bgra8Unorm,
        ] {
            assert_eq!(composite_fragment_entry(format), "fragment_gamma");
        }
        for format in [
            wgpu::TextureFormat::Rgba8UnormSrgb,
            wgpu::TextureFormat::Bgra8UnormSrgb,
        ] {
            assert_eq!(composite_fragment_entry(format), "fragment_linear");
        }

        for rgb in [
            [12, 24, 48],
            [128, 128, 128],
            [237, 28, 36],
            [255, 255, 255],
        ] as [[u8; 3]; 4]
        {
            for channel in rgb {
                let gamma = f32::from(channel) / 255.0;
                let displayed_on_gamma_surface = gamma;
                let displayed_on_srgb_surface = gamma_from_linear(linear_from_gamma(gamma));
                assert!((displayed_on_gamma_surface * 255.0 - f32::from(channel)).abs() <= 0.5);
                assert!((displayed_on_srgb_surface * 255.0 - f32::from(channel)).abs() <= 0.5);
            }
        }
    }

    #[test]
    fn solo_guides_outline_each_other_body_part_once() {
        let guides = guide_vertices(ModelKind::Classic, BodyPart::RightArm);
        assert_eq!(guides.len(), 5 * 12 * 2);
        assert!(guides.iter().all(|vertex| vertex.position[0] >= -4.0));
    }

    #[test]
    fn preview_geometry_contains_only_clipped_footprint_texels() {
        let hit = ModelHit {
            part: crate::BodyPart::Head,
            layer: Layer::Outer,
            face: Face::Front,
            distance: 1.0,
            texel: Texel::new(40, 8),
        };
        assert_eq!(
            preview_vertices(
                ModelKind::Classic,
                hit,
                BrushSize::One,
                ModelArrangement::Joined,
            )
            .len(),
            6
        );
        assert_eq!(
            preview_vertices(
                ModelKind::Classic,
                hit,
                BrushSize::Four,
                ModelArrangement::Joined,
            )
            .len(),
            9 * 6
        );
    }

    #[test]
    fn preview_geometry_respects_flipped_face_orientation() {
        let region = face_region(
            ModelKind::Classic,
            crate::BodyPart::Head,
            Layer::Base,
            Face::Left,
        );
        assert!(region.flip_u);
        let make_hit = |x| ModelHit {
            part: crate::BodyPart::Head,
            layer: Layer::Base,
            face: Face::Left,
            distance: 1.0,
            texel: Texel::new(x, region.rect.y),
        };
        let first = preview_vertices(
            ModelKind::Classic,
            make_hit(region.rect.x),
            BrushSize::One,
            ModelArrangement::Joined,
        );
        let second = preview_vertices(
            ModelKind::Classic,
            make_hit(region.rect.x + 1),
            BrushSize::One,
            ModelArrangement::Joined,
        );
        let average_z = |vertices: &[PreviewVertex]| {
            vertices
                .iter()
                .map(|vertex| vertex.position[2])
                .sum::<f32>()
                / vertices.len() as f32
        };
        assert!(average_z(&first) > average_z(&second));
    }

    #[test]
    fn exploded_preview_tracks_the_translated_body_part() {
        let hit = ModelHit {
            part: BodyPart::RightArm,
            layer: Layer::Base,
            face: Face::Left,
            distance: 1.0,
            texel: face_region(
                ModelKind::Classic,
                BodyPart::RightArm,
                Layer::Base,
                Face::Left,
            )
            .texel(0, 0)
            .unwrap(),
        };
        let joined = preview_vertices(
            ModelKind::Classic,
            hit,
            BrushSize::One,
            ModelArrangement::Joined,
        );
        let exploded = preview_vertices(
            ModelKind::Classic,
            hit,
            BrushSize::One,
            ModelArrangement::Exploded,
        );
        let offset = ModelArrangement::Exploded.offset(hit.part);
        for (joined, exploded) in joined.iter().zip(&exploded) {
            assert_eq!(
                Vec3::from(exploded.position) - Vec3::from(joined.position),
                offset
            );
        }
    }
}
