use crate::{map::Map, preferences::Preferences};
use game_core::economy::EconomyRules;
use game_protocol::{ClientCommand, ServerMessage};
use game_server::Session;
use std::{cell::RefCell, error::Error, rc::Rc, sync::Arc, time::Duration};
use wasm_bindgen::{JsCast, JsValue};
use web_time::Instant;
use winit::{
    application::ApplicationHandler,
    event::WindowEvent,
    event_loop::{ActiveEventLoop, ControlFlow, EventLoop},
    platform::web::{EventLoopExtWebSys, WindowAttributesExtWebSys, WindowExtWebSys},
    window::{Window, WindowId},
};

type Result<T> = std::result::Result<T, Box<dyn Error>>;

pub fn start() {
    if let Err(error) = run() {
        web_sys::console::error_1(&JsValue::from_str(&error.to_string()));
    }
}

fn run() -> Result<()> {
    let event_loop = EventLoop::new()?;
    let seed = crate::seed::new_seed();
    let mut authority = Session::with_economy(seed, EconomyRules::default())?;
    authority.begin_progression();
    authority.set_up_factions(seed, &Default::default())?;
    let ServerMessage::GalaxySnapshot { systems, .. } = authority.handle(ClientCommand::GetGalaxy)
    else {
        return Err("Unable to start browser game session".into());
    };
    let mut map = Map::new(systems, authority);
    map.apply_preferences(Preferences::default(), None)?;
    // Silent for now: the browser build has no audio output yet.
    let (music, _) = crate::audio::Music::open();
    map.set_music(music);
    map.notify("Browser build: saves are not persistent yet.".to_owned());

    let app = App {
        window: None,
        graphics: Rc::new(RefCell::new(None)),
        map,
        next_redraw: None,
    };
    event_loop.spawn_app(app);
    Ok(())
}

struct App {
    window: Option<Arc<Window>>,
    graphics: Rc<RefCell<Option<Graphics>>>,
    map: Map,
    next_redraw: Option<Instant>,
}

impl ApplicationHandler for App {
    fn resumed(&mut self, event_loop: &ActiveEventLoop) {
        if self.window.is_some() {
            return;
        }
        let Some(canvas) = web_sys::window()
            .and_then(|window| window.document())
            .and_then(|document| document.get_element_by_id("game-canvas"))
            .and_then(|element| element.dyn_into::<web_sys::HtmlCanvasElement>().ok())
        else {
            web_sys::console::error_1(&JsValue::from_str("Missing #game-canvas element"));
            event_loop.exit();
            return;
        };
        let window = match event_loop
            .create_window(Window::default_attributes().with_canvas(Some(canvas)))
        {
            Ok(window) => Arc::new(window),
            Err(error) => {
                web_sys::console::error_1(&JsValue::from_str(&error.to_string()));
                event_loop.exit();
                return;
            }
        };
        window.set_prevent_default(true);

        let graphics_slot = Rc::clone(&self.graphics);
        let graphics_window = Arc::clone(&window);
        wasm_bindgen_futures::spawn_local(async move {
            match Graphics::new(graphics_window.clone()).await {
                Ok(graphics) => {
                    *graphics_slot.borrow_mut() = Some(graphics);
                    graphics_window.request_redraw();
                }
                Err(error) => {
                    web_sys::console::error_1(&JsValue::from_str(&error.to_string()));
                }
            }
        });
        self.window = Some(window);
    }

    fn window_event(&mut self, event_loop: &ActiveEventLoop, id: WindowId, event: WindowEvent) {
        let Some(window) = self.window.as_ref().filter(|window| window.id() == id) else {
            return;
        };
        let mut graphics_slot = self.graphics.borrow_mut();
        let Some(graphics) = graphics_slot.as_mut() else {
            return;
        };
        let response = graphics.input.on_window_event(window, &event);
        if response.repaint {
            window.request_redraw();
        }
        match event {
            WindowEvent::Resized(_) | WindowEvent::ScaleFactorChanged { .. } => {
                graphics.resize();
                window.request_redraw();
            }
            WindowEvent::RedrawRequested => match graphics.draw(&mut self.map) {
                Ok(delay) => self.next_redraw = Instant::now().checked_add(delay),
                Err(wgpu::SurfaceError::OutOfMemory) => {
                    web_sys::console::error_1(&JsValue::from_str("GPU surface ran out of memory"));
                    event_loop.exit();
                }
                Err(wgpu::SurfaceError::Lost | wgpu::SurfaceError::Outdated) => {
                    graphics.resize();
                    window.request_redraw();
                }
                Err(error) => {
                    web_sys::console::error_1(&JsValue::from_str(&error.to_string()));
                    self.next_redraw = Some(Instant::now() + Duration::from_millis(100));
                }
            },
            _ => {}
        }
    }

    fn about_to_wait(&mut self, event_loop: &ActiveEventLoop) {
        match self.next_redraw {
            Some(deadline) if deadline <= Instant::now() => {
                self.next_redraw = None;
                if let Some(window) = &self.window {
                    window.request_redraw();
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
    async fn new(window: Arc<Window>) -> Result<Self> {
        let instance = wgpu::Instance::new(&wgpu::InstanceDescriptor::default());
        let surface = instance.create_surface(window.clone())?;
        let adapter = instance
            .request_adapter(&wgpu::RequestAdapterOptions {
                compatible_surface: Some(&surface),
                ..Default::default()
            })
            .await
            .ok_or("No compatible browser GPU adapter found")?;
        let (device, queue) = adapter
            .request_device(&wgpu::DeviceDescriptor::default(), None)
            .await?;
        let size = window.inner_size();
        let mut config = surface
            .get_default_config(&adapter, size.width.max(1), size.height.max(1))
            .ok_or("Browser GPU surface is unsupported")?;
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
                label: Some("Star Trader browser UI"),
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
