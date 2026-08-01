//! The application window, GPU renderer, and the main loop that drives them.

use std::sync::Arc;
use std::time::{Duration, Instant};

use winit::application::ApplicationHandler;
use winit::dpi::PhysicalSize;
use winit::event::WindowEvent;
use winit::event_loop::{ActiveEventLoop, ControlFlow, EventLoop, OwnedDisplayHandle};
use winit::window::{Window, WindowId};

/// Width, in physical pixels, of the window opened by [`App::run`].
const WINDOW_WIDTH: u32 = 1280;

/// Height, in physical pixels, of the window opened by [`App::run`].
const WINDOW_HEIGHT: u32 = 540;

/// Title shown in the window's title bar.
const WINDOW_TITLE: &str = "Loom";

/// Fixed interval between update ticks, decoupled from the render rate.
///
/// 60 Hz is the conventional default for a fixed-timestep game loop. There is
/// no simulation state to update yet, so this only paces the (currently
/// empty) update hook independently of how often a frame is rendered.
const UPDATE_INTERVAL: Duration = Duration::from_nanos(1_000_000_000 / 60);

/// The GPU resources needed to clear the window's surface every frame.
struct Renderer {
    /// The `wgpu` entry point used to create surfaces and request adapters.
    instance: wgpu::Instance,
    /// The open window being rendered into.
    window: Arc<Window>,
    /// The logical GPU connection used to create GPU resources.
    device: wgpu::Device,
    /// The queue commands are submitted to.
    queue: wgpu::Queue,
    /// The current size of the window's drawable surface.
    size: PhysicalSize<u32>,
    /// The drawable surface presented to the window.
    surface: wgpu::Surface<'static>,
    /// The pixel format the surface was configured with.
    surface_format: wgpu::TextureFormat,
}

impl Renderer {
    /// Creates the GPU instance, device, queue, and surface used to render
    /// into `window`.
    async fn new(display: OwnedDisplayHandle, window: Arc<Window>) -> Self {
        let instance = wgpu::Instance::new(wgpu::InstanceDescriptor::new_with_display_handle(
            Box::new(display),
        ));
        let adapter = instance
            .request_adapter(&wgpu::RequestAdapterOptions::default())
            .await
            .expect("failed to find a compatible GPU adapter");
        let (device, queue) = adapter
            .request_device(&wgpu::DeviceDescriptor::default())
            .await
            .expect("failed to open a connection to the GPU adapter");

        let size = window.inner_size();
        let surface = instance
            .create_surface(window.clone())
            .expect("failed to create a rendering surface for the window");
        let capabilities = surface.get_capabilities(&adapter);
        let surface_format = capabilities.formats[0];

        let renderer = Self {
            instance,
            window,
            device,
            queue,
            size,
            surface,
            surface_format,
        };
        renderer.configure_surface();
        renderer
    }

    /// Returns the window this renderer draws into.
    fn window(&self) -> &Window {
        &self.window
    }

    /// Configures the surface for its current size and format.
    fn configure_surface(&self) {
        let config = wgpu::SurfaceConfiguration {
            usage: wgpu::TextureUsages::RENDER_ATTACHMENT,
            format: self.surface_format,
            color_space: wgpu::SurfaceColorSpace::Auto,
            // Request compatibility with the sRGB-format texture view
            // created in `render`.
            view_formats: vec![self.surface_format.add_srgb_suffix()],
            alpha_mode: wgpu::CompositeAlphaMode::Auto,
            width: self.size.width,
            height: self.size.height,
            desired_maximum_frame_latency: 2,
            present_mode: wgpu::PresentMode::AutoVsync,
        };
        self.surface.configure(&self.device, &config);
    }

    /// Records the window's new size and reconfigures the surface to match.
    fn resize(&mut self, new_size: PhysicalSize<u32>) {
        // A minimized window reports a zero size; configuring a surface with
        // zero area panics, so wait for a real size instead.
        if new_size.width == 0 || new_size.height == 0 {
            return;
        }
        self.size = new_size;
        self.configure_surface();
    }

    /// Clears the surface to black and presents the result.
    fn render(&mut self) {
        let surface_texture = match self.surface.get_current_texture() {
            wgpu::CurrentSurfaceTexture::Success(texture) => texture,
            wgpu::CurrentSurfaceTexture::Occluded | wgpu::CurrentSurfaceTexture::Timeout => {
                return;
            }
            wgpu::CurrentSurfaceTexture::Suboptimal(texture) => {
                drop(texture);
                self.configure_surface();
                return;
            }
            wgpu::CurrentSurfaceTexture::Outdated => {
                self.configure_surface();
                return;
            }
            wgpu::CurrentSurfaceTexture::Validation => {
                unreachable!("no error scope is registered, so validation errors panic instead")
            }
            wgpu::CurrentSurfaceTexture::Lost => {
                self.surface = self
                    .instance
                    .create_surface(self.window.clone())
                    .expect("failed to recreate the rendering surface");
                self.configure_surface();
                return;
            }
        };
        let view = surface_texture
            .texture
            .create_view(&wgpu::TextureViewDescriptor {
                // Without add_srgb_suffix() the image would not be gamma
                // correct.
                format: Some(self.surface_format.add_srgb_suffix()),
                ..Default::default()
            });

        let mut encoder = self
            .device
            .create_command_encoder(&wgpu::CommandEncoderDescriptor::default());
        {
            let _render_pass = encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
                label: None,
                color_attachments: &[Some(wgpu::RenderPassColorAttachment {
                    view: &view,
                    depth_slice: None,
                    resolve_target: None,
                    ops: wgpu::Operations {
                        load: wgpu::LoadOp::Clear(wgpu::Color::BLACK),
                        store: wgpu::StoreOp::Store,
                    },
                })],
                depth_stencil_attachment: None,
                timestamp_writes: None,
                occlusion_query_set: None,
                multiview_mask: None,
            });
        }

        self.queue.submit([encoder.finish()]);
        self.window.pre_present_notify();
        self.queue.present(surface_texture);
    }
}

/// The engine's entry point.
///
/// Opens a window and runs a game loop with an update rate decoupled from
/// its render rate: the update hook ticks at a fixed 60 Hz while rendering
/// happens as fast as the platform's event loop and presentation mode
/// allow.
///
/// # Examples
///
/// ```no_run
/// use loom::prelude::*;
///
/// App::new().run();
/// ```
#[derive(Default)]
pub struct App {
    /// GPU and window resources, created once the platform resumes the app.
    renderer: Option<Renderer>,
    /// Wall-clock time of the last update tick.
    last_tick: Option<Instant>,
    /// Time accumulated since the last update tick, not yet consumed by one.
    accumulator: Duration,
}

impl App {
    /// Creates a new, unstarted application.
    pub fn new() -> Self {
        Self::default()
    }

    /// Opens the window and runs the application loop until it is closed.
    ///
    /// # Panics
    ///
    /// Panics if the platform event loop cannot be created or exits with an
    /// error.
    pub fn run(mut self) {
        let event_loop = EventLoop::new().expect("failed to create the platform event loop");
        // Render continuously rather than only in response to platform
        // events, which is what a game loop needs.
        event_loop.set_control_flow(ControlFlow::Poll);
        event_loop
            .run_app(&mut self)
            .expect("the event loop exited with an error");
    }

    /// Advances simulation state by one fixed tick.
    ///
    /// A no-op for now: game state is out of scope for this issue. This
    /// exists as the hook later work will fill in, ticking independently of
    /// [`Renderer::render`].
    fn update(&mut self) {}
}

impl ApplicationHandler for App {
    fn resumed(&mut self, event_loop: &ActiveEventLoop) {
        let attributes = Window::default_attributes()
            .with_title(WINDOW_TITLE)
            .with_inner_size(PhysicalSize::new(WINDOW_WIDTH, WINDOW_HEIGHT));
        let window = Arc::new(
            event_loop
                .create_window(attributes)
                .expect("failed to create the application window"),
        );

        let renderer = pollster::block_on(Renderer::new(
            event_loop.owned_display_handle(),
            window.clone(),
        ));
        self.renderer = Some(renderer);
        self.last_tick = Some(Instant::now());

        window.request_redraw();
    }

    fn window_event(&mut self, event_loop: &ActiveEventLoop, _id: WindowId, event: WindowEvent) {
        let Some(renderer) = self.renderer.as_mut() else {
            return;
        };
        match event {
            WindowEvent::CloseRequested => event_loop.exit(),
            WindowEvent::RedrawRequested => {
                renderer.render();
                renderer.window().request_redraw();
            }
            WindowEvent::Resized(size) => renderer.resize(size),
            _ => {}
        }
    }

    fn about_to_wait(&mut self, _event_loop: &ActiveEventLoop) {
        let Some(last_tick) = self.last_tick else {
            return;
        };
        let now = Instant::now();
        self.accumulator += now - last_tick;
        self.last_tick = Some(now);

        while self.accumulator >= UPDATE_INTERVAL {
            self.update();
            self.accumulator -= UPDATE_INTERVAL;
        }
    }
}
