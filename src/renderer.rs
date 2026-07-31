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
    Camera, Face, Layer, LayerVisibility, ModelHit, ModelKind, Skin, face_region, model_boxes,
    skin::{SKIN_HEIGHT, SKIN_WIDTH},
};

const TARGET_FORMAT: wgpu::TextureFormat = wgpu::TextureFormat::Rgba8UnormSrgb;

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
    pub hit: Option<ModelHit>,
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
    shade: f32,
    highlight: f32,
}

impl Vertex {
    const LAYOUT: wgpu::VertexBufferLayout<'static> = wgpu::VertexBufferLayout {
        array_stride: size_of::<Self>() as u64,
        step_mode: wgpu::VertexStepMode::Vertex,
        attributes: &wgpu::vertex_attr_array![
            0 => Float32x3,
            1 => Float32x2,
            2 => Float32,
            3 => Float32
        ],
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
    composite_pipeline: wgpu::RenderPipeline,
    skin_texture: wgpu::Texture,
    skin_bind_group: wgpu::BindGroup,
    scene_buffer: wgpu::Buffer,
    scene_bind_group: wgpu::BindGroup,
    vertex_buffer: wgpu::Buffer,
    vertex_capacity: usize,
    vertex_count: u32,
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
            format: TARGET_FORMAT,
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
                    format: TARGET_FORMAT,
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
                entry_point: Some("fragment_main"),
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
        let mut renderer = Self {
            model_pipeline,
            composite_pipeline,
            skin_texture,
            skin_bind_group,
            scene_buffer,
            scene_bind_group,
            vertex_buffer,
            vertex_capacity,
            vertex_count: 0,
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

        let vertices = model_vertices(callback.kind, callback.visibility, callback.hit);
        debug_assert!(vertices.len() <= self.vertex_capacity);
        queue.write_buffer(&self.vertex_buffer, 0, bytemuck::cast_slice(&vertices));
        self.vertex_count = vertices.len() as u32;
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
        pass.set_pipeline(&self.model_pipeline);
        pass.set_bind_group(0, &self.scene_bind_group, &[]);
        pass.set_bind_group(1, &self.skin_bind_group, &[]);
        pass.set_vertex_buffer(0, self.vertex_buffer.slice(..));
        pass.draw(0..self.vertex_count, 0..1);
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
            format: TARGET_FORMAT,
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

fn model_vertices(
    kind: ModelKind,
    visibility: LayerVisibility,
    hit: Option<ModelHit>,
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
        for model_box in model_boxes(kind)
            .into_iter()
            .filter(|item| item.layer == layer)
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
                let highlighted = hit.is_some_and(|item| {
                    item.part == model_box.part && item.layer == layer && item.face == face
                }) as u8 as f32;
                let shade = match face {
                    Face::Top => 1.0,
                    Face::Front | Face::Left => 0.88,
                    Face::Back | Face::Right => 0.72,
                    Face::Bottom => 0.58,
                };
                let quad = [
                    vertex(corners[0], [u0, v0], shade, highlighted),
                    vertex(corners[1], [u1, v0], shade, highlighted),
                    vertex(corners[2], [u1, v1], shade, highlighted),
                    vertex(corners[0], [u0, v0], shade, highlighted),
                    vertex(corners[2], [u1, v1], shade, highlighted),
                    vertex(corners[3], [u0, v1], shade, highlighted),
                ];
                vertices.extend_from_slice(&quad);
            }
        }
    }
    vertices
}

fn vertex(position: Vec3, uv: [f32; 2], shade: f32, highlight: f32) -> Vertex {
    Vertex {
        position: position.to_array(),
        uv,
        shade,
        highlight,
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
    @location(2) shade: f32,
    @location(3) highlight: f32,
};

struct VertexOutput {
    @builtin(position) position: vec4<f32>,
    @location(0) uv: vec2<f32>,
    @location(1) shade: f32,
    @location(2) highlight: f32,
};

@vertex
fn vertex_main(input: VertexInput) -> VertexOutput {
    var output: VertexOutput;
    output.position = scene.view_projection * vec4<f32>(input.position, 1.0);
    output.uv = input.uv;
    output.shade = input.shade;
    output.highlight = input.highlight;
    return output;
}

@fragment
fn fragment_main(input: VertexOutput) -> @location(0) vec4<f32> {
    var color = textureSample(skin, skin_sampler, input.uv);
    if color.a < 0.004 {
        discard;
    }
    let lit = vec3<f32>(color.rgb * input.shade);
    let highlighted = mix(lit, vec3<f32>(1.0, 0.78, 0.18), input.highlight * 0.26);
    return vec4<f32>(highlighted, color.a);
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

@fragment
fn fragment_main(input: VertexOutput) -> @location(0) vec4<f32> {
    return textureSample(source, source_sampler, input.uv);
}
"#;

#[cfg(test)]
mod tests {
    use super::*;
    use crate::Texel;

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
        let base = model_vertices(ModelKind::Classic, LayerVisibility::BASE_ONLY, None);
        let all = model_vertices(ModelKind::Classic, LayerVisibility::ALL, None);
        assert_eq!(base.len(), 6 * 6 * 6);
        assert_eq!(all.len(), 12 * 6 * 6);
        assert!(base.iter().all(|vertex| vertex.highlight == 0.0));
    }
}
