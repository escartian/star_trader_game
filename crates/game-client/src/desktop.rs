use crate::map::Map;
use std::{
    error::Error,
    sync::Arc,
    time::{Duration, Instant},
};
use winit::{
    application::ApplicationHandler,
    event::WindowEvent,
    event_loop::{ActiveEventLoop, ControlFlow, EventLoop},
    window::{Window, WindowId},
};

type Result<T> = std::result::Result<T, Box<dyn Error>>;

pub fn run(map: Map) -> Result<()> {
    let event_loop = EventLoop::new()?;
    let mut app = App {
        graphics: None,
        map,
        next_redraw: None,
        error: None,
    };
    event_loop.run_app(&mut app)?;
    match app.error {
        Some(error) => Err(error),
        None => Ok(()),
    }
}

struct App {
    graphics: Option<Graphics>,
    map: Map,
    next_redraw: Option<Instant>,
    error: Option<Box<dyn Error>>,
}

impl ApplicationHandler for App {
    fn resumed(&mut self, event_loop: &ActiveEventLoop) {
        if self.graphics.is_some() {
            return;
        }
        self.map.reset_clock();
        match pollster::block_on(Graphics::new(event_loop)) {
            Ok(graphics) => {
                graphics.window.request_redraw();
                self.graphics = Some(graphics);
            }
            Err(error) => {
                self.error = Some(error);
                event_loop.exit();
            }
        }
    }

    fn suspended(&mut self, _: &ActiveEventLoop) {
        self.map.reset_clock();
        self.graphics = None;
        self.next_redraw = None;
    }

    fn window_event(&mut self, event_loop: &ActiveEventLoop, id: WindowId, event: WindowEvent) {
        let Some(g) = &mut self.graphics else {
            return;
        };
        if id != g.window.id() {
            return;
        }
        let response = g.input.on_window_event(&g.window, &event);
        if response.repaint {
            g.window.request_redraw();
        }
        match event {
            WindowEvent::CloseRequested => event_loop.exit(),
            WindowEvent::Resized(_) | WindowEvent::ScaleFactorChanged { .. } => {
                g.resize();
                g.window.request_redraw();
            }
            WindowEvent::RedrawRequested => match g.draw(&mut self.map) {
                Ok(delay) => self.next_redraw = Instant::now().checked_add(delay),
                Err(wgpu::SurfaceError::OutOfMemory) => {
                    self.error = Some("GPU surface ran out of memory".into());
                    event_loop.exit();
                }
                Err(wgpu::SurfaceError::Lost | wgpu::SurfaceError::Outdated) => {
                    g.resize();
                    g.window.request_redraw();
                }
                Err(_) => self.next_redraw = Some(Instant::now() + Duration::from_millis(100)),
            },
            _ => {}
        }
    }

    fn about_to_wait(&mut self, event_loop: &ActiveEventLoop) {
        match self.next_redraw {
            Some(deadline) if deadline <= Instant::now() => {
                self.next_redraw = None;
                if let Some(g) = &self.graphics {
                    g.window.request_redraw();
                }
                event_loop.set_control_flow(ControlFlow::Wait);
            }
            Some(deadline) => event_loop.set_control_flow(ControlFlow::WaitUntil(deadline)),
            None => event_loop.set_control_flow(ControlFlow::Wait),
        }
    }
}

struct Graphics {
    window: Arc<Window>,
    surface: wgpu::Surface<'static>,
    device: wgpu::Device,
    queue: wgpu::Queue,
    config: wgpu::SurfaceConfiguration,
    context: egui::Context,
    input: egui_winit::State,
    renderer: egui_wgpu::Renderer,
}

impl Graphics {
    async fn new(event_loop: &ActiveEventLoop) -> Result<Self> {
        let window = Arc::new(
            event_loop.create_window(
                Window::default_attributes()
                    .with_title("Star Trader • Offline prototype")
                    .with_inner_size(winit::dpi::LogicalSize::new(1000.0, 720.0))
                    .with_min_inner_size(winit::dpi::LogicalSize::new(320.0, 480.0)),
            )?,
        );
        let instance = wgpu::Instance::new(&wgpu::InstanceDescriptor::default());
        let surface = instance.create_surface(window.clone())?;
        let adapter = instance
            .request_adapter(&wgpu::RequestAdapterOptions {
                compatible_surface: Some(&surface),
                ..Default::default()
            })
            .await
            .ok_or("No compatible GPU adapter found")?;
        let (device, queue) = adapter
            .request_device(&wgpu::DeviceDescriptor::default(), None)
            .await?;
        let size = window.inner_size();
        let mut config = surface
            .get_default_config(&adapter, size.width.max(1), size.height.max(1))
            .ok_or("GPU surface is unsupported")?;
        // egui's shader writes gamma-space colors.
        config.format = surface
            .get_capabilities(&adapter)
            .formats
            .into_iter()
            .find(|format| !format.is_srgb())
            .unwrap_or(config.format);
        surface.configure(&device, &config);
        let context = egui::Context::default();
        context.set_visuals(egui::Visuals::dark());
        let input = egui_winit::State::new(
            context.clone(),
            egui::ViewportId::ROOT,
            window.as_ref(),
            Some(window.scale_factor() as f32),
            None,
            None,
        );
        let renderer = egui_wgpu::Renderer::new(&device, config.format, None, 1, false);
        Ok(Self {
            window,
            surface,
            device,
            queue,
            config,
            context,
            input,
            renderer,
        })
    }

    fn resize(&mut self) {
        let size = self.window.inner_size();
        if size.width == 0 || size.height == 0 {
            return;
        }
        self.config.width = size.width;
        self.config.height = size.height;
        self.surface.configure(&self.device, &self.config);
    }

    fn draw(&mut self, map: &mut Map) -> std::result::Result<Duration, wgpu::SurfaceError> {
        let size = self.window.inner_size();
        if size.width == 0 || size.height == 0 {
            return Ok(Duration::MAX);
        }
        let frame = self.surface.get_current_texture()?;
        let output = self
            .context
            .run(self.input.take_egui_input(&self.window), |ctx| map.ui(ctx));
        self.input
            .handle_platform_output(&self.window, output.platform_output);
        let delay = output
            .viewport_output
            .get(&egui::ViewportId::ROOT)
            .map_or(Duration::MAX, |viewport| viewport.repaint_delay);
        let jobs = self
            .context
            .tessellate(output.shapes, output.pixels_per_point);
        let screen = egui_wgpu::ScreenDescriptor {
            size_in_pixels: [self.config.width, self.config.height],
            pixels_per_point: output.pixels_per_point,
        };
        for (id, delta) in &output.textures_delta.set {
            self.renderer
                .update_texture(&self.device, &self.queue, *id, delta);
        }
        let mut encoder = self.device.create_command_encoder(&Default::default());
        let buffers =
            self.renderer
                .update_buffers(&self.device, &self.queue, &mut encoder, &jobs, &screen);
        let view = frame.texture.create_view(&Default::default());
        {
            let pass = encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
                label: Some("Star Trader UI"),
                color_attachments: &[Some(wgpu::RenderPassColorAttachment {
                    view: &view,
                    resolve_target: None,
                    ops: wgpu::Operations {
                        load: wgpu::LoadOp::Clear(wgpu::Color::BLACK),
                        store: wgpu::StoreOp::Store,
                    },
                })],
                ..Default::default()
            });
            self.renderer
                .render(&mut pass.forget_lifetime(), &jobs, &screen);
        }
        self.queue
            .submit(buffers.into_iter().chain([encoder.finish()]));
        frame.present();
        for id in &output.textures_delta.free {
            self.renderer.free_texture(id);
        }
        Ok(delay)
    }
}
