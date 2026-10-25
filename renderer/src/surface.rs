//! RendererSurface — owns the wgpu device, queue, and surface.
//! Exposes a draw() method that consumes a GridSnapshot.

use wgpu::util::DeviceExt;
use std::sync::Arc;
use winit::window::Window;

use crate::{
    atlas::Atlas,
    pipeline::{CellVertex, TerminalPipeline},
    theme::Theme,
};
use vt_parser::grid::GridSnapshot;
use vt_parser::cell::Color;

const ATLAS_SIZE: u32 = 2048;

pub struct RendererSurface {
    pub device: wgpu::Device,
    pub queue: wgpu::Queue,
    pub surface: wgpu::Surface<'static>,
    pub surface_config: wgpu::SurfaceConfiguration,
    pub pipeline: TerminalPipeline,
    pub atlas: Atlas,
    pub atlas_texture: wgpu::Texture,
    pub atlas_bind_group: wgpu::BindGroup,
    pub theme: Theme,
    pub cell_width: f32,
    pub cell_height: f32,
}

impl RendererSurface {
    /// Initialize wgpu surface from an existing `winit::Window`.
    pub async fn new(window: Arc<Window>, theme: Theme, cell_width: f32, cell_height: f32) -> Self {
        let size = window.inner_size();

        let instance = wgpu::Instance::new(wgpu::InstanceDescriptor {
            backends: wgpu::Backends::all(),
            dx12_shader_compiler: Default::default(),
            flags: wgpu::InstanceFlags::default(),
            gles_minor_version: wgpu::Gles3MinorVersion::Automatic,
        });

        let surface = instance.create_surface(window).expect("Surface creation failed");

        let adapter = instance
            .request_adapter(&wgpu::RequestAdapterOptions {
                power_preference: wgpu::PowerPreference::HighPerformance,
                compatible_surface: Some(&surface),
                force_fallback_adapter: false,
            })
            .await
            .expect("No suitable GPU adapter found");

        let (device, queue) = adapter
            .request_device(
                &wgpu::DeviceDescriptor {
                    label: None,
                    required_features: wgpu::Features::empty(),
                    required_limits: wgpu::Limits::default(),
                },
                None,
            )
            .await
            .expect("Device request failed");

        let surface_caps = surface.get_capabilities(&adapter);
        let surface_format = surface_caps
            .formats
            .iter()
            .find(|f| f.is_srgb())
            .copied()
            .unwrap_or(surface_caps.formats[0]);

        let surface_config = wgpu::SurfaceConfiguration {
            usage: wgpu::TextureUsages::RENDER_ATTACHMENT,
            format: surface_format,
            width: size.width,
            height: size.height,
            present_mode: wgpu::PresentMode::Fifo,
            alpha_mode: surface_caps.alpha_modes[0],
            view_formats: vec![],
            desired_maximum_frame_latency: 2,
        };
        surface.configure(&device, &surface_config);

        let pipeline = TerminalPipeline::new(&device, surface_format);
        let atlas = Atlas::new(ATLAS_SIZE, ATLAS_SIZE);

        let atlas_texture = device.create_texture(&wgpu::TextureDescriptor {
            label: Some("Glyph Atlas"),
            size: wgpu::Extent3d {
                width: ATLAS_SIZE,
                height: ATLAS_SIZE,
                depth_or_array_layers: 1,
            },
            mip_level_count: 1,
            sample_count: 1,
            dimension: wgpu::TextureDimension::D2,
            format: wgpu::TextureFormat::Rgba8UnormSrgb,
            usage: wgpu::TextureUsages::TEXTURE_BINDING | wgpu::TextureUsages::COPY_DST,
            view_formats: &[],
        });

        let atlas_view = atlas_texture.create_view(&Default::default());
        let atlas_sampler = device.create_sampler(&wgpu::SamplerDescriptor {
            address_mode_u: wgpu::AddressMode::ClampToEdge,
            address_mode_v: wgpu::AddressMode::ClampToEdge,
            mag_filter: wgpu::FilterMode::Linear,
            min_filter: wgpu::FilterMode::Linear,
            ..Default::default()
        });

        let atlas_bind_group = device.create_bind_group(&wgpu::BindGroupDescriptor {
            label: Some("Atlas Bind Group"),
            layout: &pipeline.bind_group_layout,
            entries: &[
                wgpu::BindGroupEntry {
                    binding: 0,
                    resource: wgpu::BindingResource::TextureView(&atlas_view),
                },
                wgpu::BindGroupEntry {
                    binding: 1,
                    resource: wgpu::BindingResource::Sampler(&atlas_sampler),
                },
            ],
        });

        Self {
            device,
            queue,
            surface,
            surface_config,
            pipeline,
            atlas,
            atlas_texture,
            atlas_bind_group,
            theme,
            cell_width,
            cell_height,
        }
    }

    /// Handle window resize.
    pub fn resize(&mut self, new_width: u32, new_height: u32) {
        if new_width == 0 || new_height == 0 {
            return;
        }
        self.surface_config.width = new_width;
        self.surface_config.height = new_height;
        self.surface.configure(&self.device, &self.surface_config);
    }

    /// Render a full grid snapshot.
    pub fn draw(&mut self, snapshot: &GridSnapshot) {
        let output = match self.surface.get_current_texture() {
            Ok(o) => o,
            Err(_) => return,
        };
        let view = output.texture.create_view(&Default::default());
        let bg = self.theme.background.to_wgpu();
        let mut encoder = self.device.create_command_encoder(&Default::default());

        // Build vertex buffer from grid cells
        let mut vertices: Vec<CellVertex> = Vec::with_capacity(snapshot.rows * snapshot.cols * 6);
        let sw = self.surface_config.width as f32;
        let sh = self.surface_config.height as f32;
        let cw = self.cell_width;
        let ch = self.cell_height;

        for row in 0..snapshot.rows {
            for col in 0..snapshot.cols {
                let cell = &snapshot.cells[row][col];
                if cell.wide_placeholder {
                    continue;
                }

                let x0 = (col as f32 * cw / (sw / 2.0)) - 1.0;
                let y0 = 1.0 - (row as f32 * ch / (sh / 2.0));
                let x1 = x0 + cw / (sw / 2.0);
                let y1 = y0 - ch / (sh / 2.0);

                let fg = self.resolve_color(&cell.fg, false);
                let bg_cell = self.resolve_color(&cell.bg, true);

                let quad = [
                    CellVertex { position: [x0, y0], uv: [0.0, 0.0], fg_color: fg, bg_color: bg_cell },
                    CellVertex { position: [x1, y0], uv: [1.0, 0.0], fg_color: fg, bg_color: bg_cell },
                    CellVertex { position: [x0, y1], uv: [0.0, 1.0], fg_color: fg, bg_color: bg_cell },
                    CellVertex { position: [x1, y0], uv: [1.0, 0.0], fg_color: fg, bg_color: bg_cell },
                    CellVertex { position: [x1, y1], uv: [1.0, 1.0], fg_color: fg, bg_color: bg_cell },
                    CellVertex { position: [x0, y1], uv: [0.0, 1.0], fg_color: fg, bg_color: bg_cell },
                ];
                vertices.extend_from_slice(&quad);
            }
        }

        let vb = self.device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
            label: Some("Cell Vertex Buffer"),
            contents: bytemuck::cast_slice(&vertices),
            usage: wgpu::BufferUsages::VERTEX,
        });

        {
            let mut rpass = encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
                label: Some("Terminal Render Pass"),
                color_attachments: &[Some(wgpu::RenderPassColorAttachment {
                    view: &view,
                    resolve_target: None,
                    ops: wgpu::Operations {
                        load: wgpu::LoadOp::Clear(bg),
                        store: wgpu::StoreOp::Store,
                    },
                })],
                depth_stencil_attachment: None,
                timestamp_writes: None,
                occlusion_query_set: None,
            });

            rpass.set_pipeline(&self.pipeline.render_pipeline);
            rpass.set_bind_group(0, &self.atlas_bind_group, &[]);
            rpass.set_vertex_buffer(0, vb.slice(..));
            rpass.draw(0..vertices.len() as u32, 0..1);
        }

        self.queue.submit(std::iter::once(encoder.finish()));
        output.present();
    }

    fn resolve_color(&self, color: &Color, is_bg: bool) -> [f32; 4] {
        match color {
            Color::Default => {
                if is_bg {
                    let c = &self.theme.background;
                    [c.0 as f32, c.1 as f32, c.2 as f32, c.3 as f32]
                } else {
                    let c = &self.theme.foreground;
                    [c.0 as f32, c.1 as f32, c.2 as f32, c.3 as f32]
                }
            }
            Color::Indexed(i) => {
                let c = &self.theme.ansi[(*i as usize).min(15)];
                [c.0 as f32, c.1 as f32, c.2 as f32, c.3 as f32]
            }
            Color::Rgb(r, g, b) => [
                *r as f32 / 255.0,
                *g as f32 / 255.0,
                *b as f32 / 255.0,
                1.0,
            ],
        }
    }
}
