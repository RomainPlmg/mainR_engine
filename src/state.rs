use crate::{
    camera::{CameraController, CameraResource},
    gpu_context::{GpuContext, WindowSurface},
    player::{Player, PlayerController},
    world::{self, World, WorldResource},
};
use std::sync::Arc;
use winit::window::Window;

pub struct State {
    pub display: WindowSurface,
    render_pipeline: wgpu::RenderPipeline,
    gpu: GpuContext,

    // Player
    pub player_controller: PlayerController,
    player: Player,

    // Camera
    camera_resource: CameraResource,
    pub camera_controller: CameraController,

    // World
    world: World,
    world_resource: WorldResource,
}

impl State {
    pub async fn new(window: Arc<Window>) -> anyhow::Result<Self> {
        let (gpu, display) = GpuContext::new(window).await?;

        let player = Player::new(glam::Vec3::new(0.0, world::GRID_SIZE as f32, 0.0));
        let player_controller = PlayerController::default();
        let camera_resource = CameraResource::new(&gpu.device, &player.camera);
        let camera_controller = CameraController::new(0.1);

        let world = World::new();
        let world_resource = WorldResource::new(&gpu.device, &world);

        // Confiure the render pipeline
        let shader = gpu
            .device
            .create_shader_module(wgpu::include_wgsl!("shader.wgsl"));
        let render_pipeline_layout =
            gpu.device
                .create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
                    label: Some("Render Pipeline Layout"),
                    bind_group_layouts: &[
                        &camera_resource.layout, // @group(0)
                        &world_resource.layout,  // @group(1)
                    ],
                    immediate_size: 0,
                });
        let render_pipeline = gpu
            .device
            .create_render_pipeline(&wgpu::RenderPipelineDescriptor {
                label: Some("Render Pipeline"),
                layout: Some(&render_pipeline_layout),
                vertex: wgpu::VertexState {
                    module: &shader,
                    entry_point: Some("vs_main"),
                    buffers: &[],
                    compilation_options: wgpu::PipelineCompilationOptions::default(),
                },
                fragment: Some(wgpu::FragmentState {
                    // 3.
                    module: &shader,
                    entry_point: Some("fs_main"),
                    targets: &[Some(wgpu::ColorTargetState {
                        // 4.
                        format: display.config.format,
                        blend: Some(wgpu::BlendState::REPLACE),
                        write_mask: wgpu::ColorWrites::ALL,
                    })],
                    compilation_options: wgpu::PipelineCompilationOptions::default(),
                }),
                primitive: wgpu::PrimitiveState {
                    topology: wgpu::PrimitiveTopology::TriangleList,
                    strip_index_format: None,
                    front_face: wgpu::FrontFace::Ccw,
                    cull_mode: Some(wgpu::Face::Back),
                    polygon_mode: wgpu::PolygonMode::Fill,
                    unclipped_depth: false,
                    conservative: false,
                },
                depth_stencil: None,
                multisample: wgpu::MultisampleState {
                    count: 1,
                    mask: !0,
                    alpha_to_coverage_enabled: false,
                },
                multiview_mask: None,
                cache: None,
            });

        Ok(Self {
            gpu,
            display,
            render_pipeline,
            player_controller,
            player,
            camera_resource,
            camera_controller,
            world,
            world_resource,
        })
    }

    pub fn update(&mut self, dt: std::time::Duration) {
        let size = glam::USizeVec2::new(
            self.display.config.width as usize,
            self.display.config.height as usize,
        );

        self.camera_controller
            .update_camera(&mut self.player.camera, size.x, size.y);
        self.player.move_player(&self.player_controller, dt, 3.0);
        self.camera_resource
            .update(&self.gpu.queue, &self.player.camera);
    }

    pub fn resize(&mut self, width: u32, height: u32) {
        self.display.resize(&self.gpu.device, width, height);
    }

    pub fn render(&mut self) -> Result<(), wgpu::SurfaceError> {
        let output = self.display.surface.get_current_texture()?;
        let view = output
            .texture
            .create_view(&wgpu::TextureViewDescriptor::default());

        let mut encoder = self
            .gpu
            .device
            .create_command_encoder(&wgpu::CommandEncoderDescriptor {
                label: Some("Render Encoder"),
            });

        {
            let mut render_pass = encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
                label: Some("Render Pass"),
                color_attachments: &[Some(wgpu::RenderPassColorAttachment {
                    view: &view,
                    resolve_target: None,
                    depth_slice: None,
                    ops: wgpu::Operations {
                        load: wgpu::LoadOp::Clear(wgpu::Color {
                            r: 0.1,
                            g: 0.2,
                            b: 0.3,
                            a: 1.0,
                        }),
                        store: wgpu::StoreOp::Store,
                    },
                })],
                depth_stencil_attachment: None,
                occlusion_query_set: None,
                timestamp_writes: None,
                multiview_mask: None,
            });

            render_pass.set_pipeline(&self.render_pipeline);
            render_pass.set_bind_group(0, &self.camera_resource.bind_group, &[]);
            render_pass.set_bind_group(1, &self.world_resource.bind_group, &[]);
            render_pass.draw(0..3, 0..1);
        }

        self.gpu.queue.submit(std::iter::once(encoder.finish()));
        output.present();

        Ok(())
    }
}
