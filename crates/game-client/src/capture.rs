//! Render the actual UI to a GPU texture for visual review without a desktop window.
use crate::map::Map;
use std::{error::Error, path::Path};

/// Default capture size in pixels (one pixel per point).
pub const DEFAULT_SIZE: (u32, u32) = (1024, 768);
const FORMAT: wgpu::TextureFormat = wgpu::TextureFormat::Rgba8Unorm;

/// Renders the UI at `size` (pixels) and saves it as a PNG.
pub async fn save(map: &mut Map, path: &Path, size: (u32, u32)) -> Result<(), Box<dyn Error>> {
    let (width, height) = size;
    // Texture rows copy in multiples of 256 bytes; pad, then drop the padding.
    let row = width * 4;
    let padded_row = row.div_ceil(256) * 256;
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
    // egui sizes new windows/areas over multiple frames; combine their texture uploads.
    for frame in 0..6 {
        output.append(context.run(
            egui::RawInput {
                screen_rect: Some(egui::Rect::from_min_size(
                    egui::Pos2::ZERO,
                    egui::vec2(width as f32, height as f32),
                )),
                time: Some(f64::from(frame) / 60.0),
                ..Default::default()
            },
            |ctx| map.ui(ctx),
        ));
    }
    let jobs = context.tessellate(output.shapes, output.pixels_per_point);
    let screen = egui_wgpu::ScreenDescriptor {
        size_in_pixels: [width, height],
        pixels_per_point: output.pixels_per_point,
    };
    let mut renderer = egui_wgpu::Renderer::new(&device, FORMAT, None, 1, false);
    for (id, delta) in &output.textures_delta.set {
        renderer.update_texture(&device, &queue, *id, delta);
    }
    let texture = device.create_texture(&wgpu::TextureDescriptor {
        label: Some("UI capture"),
        size: wgpu::Extent3d {
            width,
            height,
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
        size: u64::from(padded_row * height),
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
                bytes_per_row: Some(padded_row),
                rows_per_image: Some(height),
            },
        },
        wgpu::Extent3d {
            width,
            height,
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
        let mapped = slice.get_mapped_range();
        let pixels: Vec<u8> = mapped
            .chunks(padded_row as usize)
            .flat_map(|line| &line[..row as usize])
            .copied()
            .collect();
        image::save_buffer(path, &pixels, width, height, image::ColorType::Rgba8)?;
    }
    readback.unmap();
    println!(
        "Captured {} using {}",
        path.display(),
        adapter.get_info().name
    );
    Ok(())
}
