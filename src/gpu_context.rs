use std::sync::Arc;
use winit::window::Window;

pub struct GpuContext {
    pub instance: wgpu::Instance,
    pub adapter: wgpu::Adapter,
    pub device: wgpu::Device,
    pub queue: wgpu::Queue,
}

pub struct WindowSurface {
    pub window: Arc<Window>,
    pub surface: wgpu::Surface<'static>,
    pub config: wgpu::SurfaceConfiguration,
}

impl GpuContext {
    pub async fn new(window: Arc<Window>) -> anyhow::Result<(Self, WindowSurface)> {
        let size = window.inner_size();

        // The instance is a handle to the GPU
        let instance = wgpu::Instance::new(&wgpu::InstanceDescriptor::default());
        let surface = instance.create_surface(window.clone()).unwrap();

        let adapter = instance
            .request_adapter(&wgpu::RequestAdapterOptions {
                compatible_surface: Some(&surface),
                power_preference: wgpu::PowerPreference::HighPerformance,
                force_fallback_adapter: false,
            })
            .await?; // Wait the GPU response (asynchronous function)

        let (device, queue) = adapter
            .request_device(&wgpu::DeviceDescriptor {
                label: None,
                required_features: wgpu::Features::empty(),
                experimental_features: wgpu::ExperimentalFeatures::disabled(),
                ..Default::default()
            })
            .await?;

        // Configure the surface (the screen)
        let surface_caps = surface.get_capabilities(&adapter);
        let surface_format = surface_caps
            .formats
            .iter()
            .find(|f| f.is_srgb())
            .copied()
            .unwrap_or(surface_caps.formats[0]);
        let config = wgpu::SurfaceConfiguration {
            alpha_mode: surface_caps.alpha_modes[0],
            desired_maximum_frame_latency: 2, // Double buffering
            format: surface_format,           // Preferred sRGB
            height: size.height,
            width: size.width,
            present_mode: surface_caps.present_modes[0],
            usage: wgpu::TextureUsages::RENDER_ATTACHMENT, // Define the surface as a canvas
            view_formats: vec![],
        };

        Ok((
            Self {
                instance,
                adapter,
                device,
                queue,
            },
            WindowSurface {
                window,
                surface,
                config,
            },
        ))
    }
}

impl WindowSurface {
    pub fn resize(&mut self, device: &wgpu::Device, width: u32, height: u32) {
        if width > 0 && height > 0 {
            self.config.width = width;
            self.config.height = height;
            self.surface.configure(device, &self.config);
        }
    }
}
