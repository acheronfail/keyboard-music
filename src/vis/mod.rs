mod renderer;

use std::collections::VecDeque;
use std::sync::mpsc::{Receiver, Sender};
use std::sync::{Arc, Mutex};
use std::thread;

use glutin::config::ConfigTemplateBuilder;
use glutin::context::{
    ContextApi,
    ContextAttributesBuilder,
    NotCurrentContext,
    PossiblyCurrentContext,
    Version,
};
use glutin::display::{Display, GetGlDisplay};
use glutin::prelude::*;
use glutin::surface::{Surface, WindowSurface};
use glutin_winit::{DisplayBuilder, GlWindow};
use raw_window_handle::HasRawWindowHandle;
use winit::dpi::PhysicalPosition;
use winit::event::{
    ElementState,
    Event,
    MouseButton,
    MouseScrollDelta,
    VirtualKeyCode,
    WindowEvent,
};
use winit::event_loop::{ControlFlow, EventLoop, EventLoopBuilder};
use winit::window::{Window, WindowBuilder};

use crate::Action;

const WINDOW_TITLE: &str = "keyboard-music";
const WINDOW_X: i32 = 635;
const WINDOW_Y: i32 = 50;
const WINDOW_WIDTH: i32 = 1600;
const WINDOW_HEIGHT: i32 = 300;

const VIS_BUFFER_MIN: f32 = 100.0;
const VIS_BUFFER_MAX: f32 = 30_000.0;
const VIS_BUFFER_DEFAULT: usize = 10_000;

/// Mostly all taken from:
/// https://github.com/rust-windowing/glutin/blob/master/glutin_examples/src/lib.rs
fn prepare_gl_window() -> (
    Window,
    EventLoop<()>,
    Display,
    Surface<WindowSurface>,
    Option<NotCurrentContext>,
) {
    let event_loop = EventLoopBuilder::new().build();
    let window_builder = WindowBuilder::new()
        .with_position(PhysicalPosition::new(WINDOW_X, WINDOW_Y))
        .with_title(WINDOW_TITLE)
        .with_inner_size(winit::dpi::LogicalSize::new(WINDOW_WIDTH, WINDOW_HEIGHT));

    let (window, gl_config) = DisplayBuilder::new()
        .with_window_builder(Some(window_builder))
        .build(&event_loop, ConfigTemplateBuilder::new(), |targets| {
            // Find the config with the maximum number of samples
            targets
                .reduce(|curr, next| {
                    let transparency_check = next.supports_transparency().unwrap_or(false)
                        && !curr.supports_transparency().unwrap_or(false);

                    if transparency_check || next.num_samples() > curr.num_samples() {
                        next
                    } else {
                        curr
                    }
                })
                .unwrap()
        })
        .unwrap();

    let window = window.expect("failed to create window");
    let gl_display = gl_config.display();

    let attrs = window.build_surface_attributes(<_>::default());
    let gl_surface = unsafe {
        gl_display
            .create_window_surface(&gl_config, &attrs)
            .unwrap()
    };

    let raw_window_handle = Some(window.raw_window_handle());

    // The context creation part. It can be created before surface and that's how
    // it's expected in multithreaded + multiwindow operation mode, since you
    // can send NotCurrentContext, but not Surface.
    let context_attributes = ContextAttributesBuilder::new().build(raw_window_handle);

    // Since glutin by default tries to create OpenGL core context, which may not be
    // present we should try gles.
    let fallback_context_attributes = ContextAttributesBuilder::new()
        .with_context_api(ContextApi::Gles(None))
        .build(raw_window_handle);

    // There are also some old devices that support neither modern OpenGL nor GLES.
    // To support these we can try and create a 2.1 context.
    let legacy_context_attributes = ContextAttributesBuilder::new()
        .with_context_api(ContextApi::OpenGl(Some(Version::new(2, 1))))
        .build(raw_window_handle);

    // Finally, we can create the gl context
    let not_current_gl_context: Option<glutin::context::NotCurrentContext> = Some(unsafe {
        gl_display
            .create_context(&gl_config, &context_attributes)
            .unwrap_or_else(|_| {
                gl_display
                    .create_context(&gl_config, &fallback_context_attributes)
                    .unwrap_or_else(|_| {
                        gl_display
                            .create_context(&gl_config, &legacy_context_attributes)
                            .expect("failed to create context")
                    })
            })
    });

    (
        window,
        event_loop,
        gl_display,
        gl_surface,
        not_current_gl_context,
    )
}

pub struct VisualiserState {
    gl_context: Option<PossiblyCurrentContext>,

    paused: bool,
    vis_buffer_size: Arc<Mutex<usize>>,
}

impl VisualiserState {
    fn toggle_pause(&mut self) {
        self.paused = !self.paused;
    }
}

pub fn open_and_run(audio_rx: Receiver<Vec<f32>>, wave_tx: Sender<Action>) -> ! {
    let audio_data = Arc::new(Mutex::new(VecDeque::with_capacity(VIS_BUFFER_DEFAULT)));
    let vis_buffer_size = Arc::new(Mutex::new(VIS_BUFFER_DEFAULT));

    let mut state = VisualiserState {
        gl_context: None,
        paused: false,
        vis_buffer_size: vis_buffer_size.clone(),
    };

    // thread which receives audio data from audio thread and copies it so we can render it
    let (win_tx, win_rx) = oneshot::channel::<Window>();
    thread::spawn({
        let audio_data = audio_data.clone();
        move || {
            let window = win_rx.recv().unwrap();
            while let Ok(audio_buf) = audio_rx.recv() {
                let mut vec = audio_data.lock().unwrap();
                vec.extend(audio_buf);
                while vec.len() > usize::max(1, *vis_buffer_size.lock().unwrap()) {
                    vec.pop_front();
                }

                window.request_redraw();
            }
        }
    });

    // create window and setup gl context
    let (window, event_loop, gl_display, gl_surface, mut not_current_gl_context) =
        prepare_gl_window();
    // send window handle to above thread so it can communicate to it
    win_tx.send(window).unwrap();

    // surrender this thread to the window's event loop and run have it take over
    let mut gl_program = None;
    event_loop.run(move |event, _, control_flow| {
        *control_flow = ControlFlow::Wait;

        match event {
            Event::LoopDestroyed => return,
            Event::WindowEvent { event, .. } => match event {
                WindowEvent::CloseRequested => *control_flow = ControlFlow::Exit,
                WindowEvent::MouseInput { state, button, .. }
                    if button == MouseButton::Left && state == ElementState::Pressed =>
                {
                    let _ = wave_tx.send(Action::NextWave);
                }
                WindowEvent::MouseWheel { delta, .. } => {
                    let (_, y) = match delta {
                        MouseScrollDelta::LineDelta(x, y) => (x, y),
                        MouseScrollDelta::PixelDelta(p) => (p.x as f32, p.y as f32),
                    };

                    let base = 1.03_f32.ln();
                    let mut g = state.vis_buffer_size.lock().unwrap();
                    let x = *g as f32;
                    *g = if y > 0.0 {
                        (x + (x.ln() / base)).min(VIS_BUFFER_MAX) as usize
                    } else {
                        (x - (x.ln() / base)).max(VIS_BUFFER_MIN) as usize
                    };
                }
                WindowEvent::KeyboardInput { input, .. } => match input.virtual_keycode {
                    // close and exit when escape is pressed
                    Some(VirtualKeyCode::Escape) => *control_flow = ControlFlow::Exit,
                    // pause waveform render when space is pressed
                    Some(VirtualKeyCode::Space) if input.state == ElementState::Pressed => {
                        state.toggle_pause()
                    }

                    _ => {}
                },
                _ => (),
            },
            Event::Resumed => {
                let gl_context = not_current_gl_context
                    .take()
                    .unwrap()
                    .make_current(&gl_surface)
                    .unwrap();

                gl_program = Some(renderer::Renderer::new(&gl_display, audio_data.clone()));
                state.gl_context = Some(gl_context);
            }
            Event::Suspended => {
                let gl_context = state.gl_context.take().unwrap();
                assert!(not_current_gl_context
                    .replace(gl_context.make_not_current().unwrap())
                    .is_none());
            }
            Event::RedrawRequested(_) => match (&state.gl_context, &mut gl_program) {
                (Some(gl_context), Some(gl_program)) => {
                    gl_program.draw(&state);
                    gl_surface.swap_buffers(&gl_context).unwrap();
                }
                _ => {}
            },
            _ => (),
        }
    })
}
