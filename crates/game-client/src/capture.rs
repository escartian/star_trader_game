//! Render the actual UI to a GPU texture for visual review without a desktop window.
use crate::map::Map;
use std::{error::Error, path::Path};

const WIDTH: u32 = 1024;
const HEIGHT: u32 = 768;
const FORMAT: wgpu::TextureFormat = wgpu::TextureFormat::Rgba8Unorm;

pub async fn save(map: &mut Map, path: &Path) -> Result<(), Box<dyn Error>> {
    let instance = wgpu::Instance::new(&wgpu::InstanceDescriptor::default());
    let adapter = instance
        .request_adapter(&wgpu::RequestAdapterOptions::default())
        .await
        .ok_or("No GPU adapter for capture")?;
    let (device, queue) = adapter
        .request_device(&wgpu::DeviceDescriptor::default(), None)
        .await?;
    let context = egui::Context::default();
    context.set_visuals(egui::Visuals::dark());
    context.style_mut(|style| style.animation_time = 0.0);
    let mut output = egui::FullOutput::default();
    // egui sizes new windows over multiple frames; combine their texture uploads.
    for frame in 0..4 {
        output.append(context.run(
            egui::RawInput {
                screen_rect: Some(egui::Rect::from_min_size(
                    egui::Pos2::ZERO,
                    egui::vec2(WIDTH as f32, HEIGHT as f32),
                )),
                time: Some(f64::from(frame) / 60.0),
                ..Default::default()
            },
            |ctx| map.ui(ctx),
        ));
    }
    let jobs = context.tessellate(output.shapes, output.pixels_per_point);
    let screen = egui_wgpu::ScreenDescriptor {
        size_in_pixels: [WIDTH, HEIGHT],
        pixels_per_point: output.pixels_per_point,
    };
    let mut renderer = egui_wgpu::Renderer::new(&device, FORMAT, None, 1, false);
    for (id, delta) in &output.textures_delta.set {
        renderer.update_texture(&device, &queue, *id, delta);
    }
    let texture = device.create_texture(&wgpu::TextureDescriptor {
        label: Some("UI capture"),
        size: wgpu::Extent3d {
            width: WIDTH,
            height: HEIGHT,
            depth_or_array_layers: 1,
        },
        mip_level_count: 1,
        sample_count: 1,
        dimension: wgpu::TextureDimension::D2,
        format: FORMAT,
        usage: wgpu::TextureUsages::RENDER_ATTACHMENT | wgpu::TextureUsages::COPY_SRC,
        view_formats: &[],
    });
    let readback = device.create_buffer(&wgpu::BufferDescriptor {
        label: Some("Capture readback"),
        size: u64::from(WIDTH * HEIGHT * 4),
        usage: wgpu::BufferUsages::COPY_DST | wgpu::BufferUsages::MAP_READ,
        mapped_at_creation: false,
    });
    let mut encoder = device.create_command_encoder(&Default::default());
    let buffers = renderer.update_buffers(&device, &queue, &mut encoder, &jobs, &screen);
    let view = texture.create_view(&Default::default());
    {
        let pass = encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
            label: Some("Capture UI pass"),
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
        renderer.render(&mut pass.forget_lifetime(), &jobs, &screen);
    }
    // WIDTH * 4 is a multiple of wgpu's 256-byte row alignment.
    encoder.copy_texture_to_buffer(
        wgpu::TexelCopyTextureInfo {
            texture: &texture,
            mip_level: 0,
            origin: wgpu::Origin3d::ZERO,
            aspect: wgpu::TextureAspect::All,
        },
        wgpu::TexelCopyBufferInfo {
            buffer: &readback,
            layout: wgpu::TexelCopyBufferLayout {
                offset: 0,
                bytes_per_row: Some(WIDTH * 4),
                rows_per_image: Some(HEIGHT),
            },
        },
        wgpu::Extent3d {
            width: WIDTH,
            height: HEIGHT,
            depth_or_array_layers: 1,
        },
    );
    queue.submit(buffers.into_iter().chain([encoder.finish()]));
    let slice = readback.slice(..);
    let (send, receive) = std::sync::mpsc::channel();
    slice.map_async(wgpu::MapMode::Read, move |result| {
        let _ = send.send(result);
    });
    let _ = device.poll(wgpu::Maintain::Wait);
    receive.recv()??;
    {
        let pixels = slice.get_mapped_range();
        image::save_buffer(path, &pixels, WIDTH, HEIGHT, image::ColorType::Rgba8)?;
    }
    readback.unmap();
    println!(
        "Captured {} using {}",
        path.display(),
        adapter.get_info().name
    );
    Ok(())
}
