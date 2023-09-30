use std::collections::VecDeque;
use std::ffi::CString;
use std::mem::size_of;
use std::sync::mpsc::Receiver;
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
use glutin::display::GetGlDisplay;
use glutin::prelude::*;
use glutin::surface::{Surface, WindowSurface};
use glutin_winit::{DisplayBuilder, GlWindow};
use raw_window_handle::HasRawWindowHandle;
use winit::dpi::PhysicalPosition;
use winit::event::{ElementState, Event, VirtualKeyCode, WindowEvent};
use winit::event_loop::{ControlFlow, EventLoop, EventLoopBuilder};
use winit::window::{Window, WindowBuilder};

use crate::notes::MAX_VOLUME;

const WINDOW_TITLE: &str = "keyboard-music";
const WINDOW_X: i32 = 635;
const WINDOW_Y: i32 = 50;
const WINDOW_WIDTH: i32 = 1600;
const WINDOW_HEIGHT: i32 = 300;

const VIS_BUFFER_SIZE: usize = 15_000;

/// Mostly all taken from:
/// https://github.com/rust-windowing/glutin/blob/master/glutin_examples/src/lib.rs
fn prepare_gl_window() -> (
    Window,
    EventLoop<()>,
    Option<NotCurrentContext>,
    Surface<WindowSurface>,
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

    // load gl function pointers to link gl to our newly created display
    let gl_display = gl_config.display();
    gl::load_with(|symbol| {
        let symbol = CString::new(symbol).unwrap();
        gl_display.get_proc_address(symbol.as_c_str()).cast()
    });

    let attrs = window.build_surface_attributes(<_>::default());
    let gl_surface = unsafe {
        gl_config
            .display()
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

    (window, event_loop, not_current_gl_context, gl_surface)
}

// TODO: make this window pausable and scrollable to be able to inspect it? or slow mo?
pub fn open_and_run(audio_rx: Receiver<Vec<f32>>) -> ! {
    let shared_data = Arc::new(Mutex::new(VecDeque::with_capacity(VIS_BUFFER_SIZE)));
    let mut gl_draw = create_gl_draw(shared_data.clone(), MAX_VOLUME);

    // thread which receives audio data from audio thread and copies it so we can render it
    let (win_tx, win_rx) = oneshot::channel::<Window>();
    thread::spawn(move || {
        let window = win_rx.recv().unwrap();
        while let Ok(audio_buf) = audio_rx.recv() {
            let mut vec = shared_data.lock().unwrap();
            vec.extend(audio_buf);
            while vec.len() > VIS_BUFFER_SIZE {
                vec.pop_front();
            }

            window.request_redraw();
        }
    });

    // create window and setup gl context
    let (window, event_loop, mut not_current_gl_context, gl_surface) = prepare_gl_window();
    // send window handle to above thread so it can communicate to it
    win_tx.send(window).unwrap();

    // surrender this thread to the window's event loop and run have it take over
    let mut state: Option<(PossiblyCurrentContext, bool)> = None;
    let mut gl_program = None;
    event_loop.run(move |event, _, control_flow| {
        *control_flow = ControlFlow::Wait;

        match event {
            Event::LoopDestroyed => return,
            Event::WindowEvent { event, .. } => match event {
                WindowEvent::CloseRequested => *control_flow = ControlFlow::Exit,
                WindowEvent::MouseWheel { delta, .. } => {
                    // TODO: scroll audio or something? (requires we keep track of a "view" of a larger data set)
                    dbg!(delta);
                }
                WindowEvent::KeyboardInput { input, .. } => match input.virtual_keycode {
                    // close and exit when escape is pressed
                    Some(VirtualKeyCode::Escape) => *control_flow = ControlFlow::Exit,
                    // pause waveform render when space is pressed
                    Some(VirtualKeyCode::Space) if input.state == ElementState::Pressed => {
                        state = match state.take() {
                            Some((x, paused)) => Some((x, !paused)),
                            None => None,
                        };
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

                // NOTE: set up the shader program here
                unsafe {
                    let vertex_source = CString::new(include_str!("./vertex.vert")).unwrap();
                    let fragment_source = CString::new(include_str!("./fragment.glsl")).unwrap();

                    // Compile the vertex shader
                    let vertex_shader = gl::CreateShader(gl::VERTEX_SHADER);
                    gl::ShaderSource(vertex_shader, 1, &vertex_source.as_ptr(), std::ptr::null());
                    gl::CompileShader(vertex_shader);

                    // Compile the fragment shader
                    let fragment_shader = gl::CreateShader(gl::FRAGMENT_SHADER);
                    gl::ShaderSource(
                        fragment_shader,
                        1,
                        &fragment_source.as_ptr(),
                        std::ptr::null(),
                    );
                    gl::CompileShader(fragment_shader);

                    // Link the vertex and fragment shader into a shader program
                    let shader_program = gl::CreateProgram();
                    gl::AttachShader(shader_program, vertex_shader);
                    gl::AttachShader(shader_program, fragment_shader);
                    gl::LinkProgram(shader_program);

                    // Delete the shaders as they're linked into our program now and no longer necessary
                    gl::DeleteShader(vertex_shader);
                    gl::DeleteShader(fragment_shader);

                    // Use the linked shader program
                    gl::UseProgram(shader_program);

                    // save this for later, so we can setup uniforms and such
                    gl_program = Some(shader_program);
                }
                // NOTE: query line width and set it here
                unsafe {
                    // let mut line_width_range = [0.0f32, 0.0f32];
                    // gl::GetFloatv(gl::ALIASED_LINE_WIDTH_RANGE, line_width_range.as_mut_ptr());
                    // // set
                    // gl::LineWidth(line_width_range[1]);
                    gl::LineWidth(2.0);
                }

                state = Some((gl_context, false));
            }
            Event::Suspended => {
                let (gl_context, ..) = state.take().unwrap();
                assert!(not_current_gl_context
                    .replace(gl_context.make_not_current().unwrap())
                    .is_none());
            }
            Event::RedrawRequested(_) => {
                if let Some((ref gl_context, paused, ..)) = state {
                    gl_draw(gl_program.unwrap(), paused);
                    gl_surface.swap_buffers(&gl_context).unwrap();
                }
            }
            _ => (),
        }
    })
}

fn create_gl_draw(
    shared_data: Arc<Mutex<VecDeque<f32>>>,
    audio_max_volume: f32,
) -> impl FnMut(u32, bool) {
    let mut audio_data_len = 0.0;
    let mut vertices = vec![];

    move |gl_program, paused| unsafe {
        // Set the clearing color to black (R=0, G=0, B=0) with full opacity (A=1.0)
        gl::ClearColor(0.0, 0.0, 0.0, 1.0);
        // Clear the color buffer with the set color
        gl::Clear(gl::COLOR_BUFFER_BIT);

        // setup uniforms
        // TODO: set these up before and re-use them, don't set them up each time on each frame draw
        let audio_data_len_s = CString::new("audio_data_len").unwrap();
        let audio_data_len_loc = gl::GetUniformLocation(gl_program, audio_data_len_s.as_ptr());
        gl::Uniform1f(audio_data_len_loc, audio_data_len);
        let audio_max_volume_s = CString::new("audio_max_volume").unwrap();
        let audio_max_volume_loc = gl::GetUniformLocation(gl_program, audio_max_volume_s.as_ptr());
        gl::Uniform1f(audio_max_volume_loc, audio_max_volume);

        let rendering_wave_s = CString::new("rendering_wave").unwrap();
        let rendering_wave_loc = gl::GetUniformLocation(gl_program, rendering_wave_s.as_ptr());

        // fetch and prepare data to be sent to gl
        if paused {
            gl::Uniform1i(rendering_wave_loc, 0);
            render_pause_icon();
        } else {
            let shared_data = shared_data.lock().unwrap();
            audio_data_len = shared_data.len() as f32;
            vertices = shared_data
                .iter()
                .enumerate()
                // we send the index and the audio value (y) over to the shader
                .flat_map(|(idx, y)| vec![idx as f32, *y])
                .collect();
        };

        gl::Uniform1i(rendering_wave_loc, 1);

        // Prepare variables for the Vertex Buffer Object (vbo) and Vertex Array Object (vao)
        let mut vbo: gl::types::GLuint = 0;
        let mut vao: gl::types::GLuint = 0;

        // Generate the buffer for vertices and bind it to the vbo
        gl::GenBuffers(1, &mut vbo);
        gl::BindBuffer(gl::ARRAY_BUFFER, vbo);

        // Send the vertex data to the GPU, specifying it's a static draw (not expected to change)
        gl::BufferData(
            gl::ARRAY_BUFFER,
            (vertices.len() * size_of::<f32>()) as gl::types::GLsizeiptr,
            &vertices[0] as *const f32 as *const gl::types::GLvoid,
            gl::STATIC_DRAW,
        );

        // Generate and bind a Vertex Array Object (vao)
        gl::GenVertexArrays(1, &mut vao);
        gl::BindVertexArray(vao);

        // Enable the first attribute of the vertex shader (at location = 0)
        gl::EnableVertexAttribArray(0);

        // Set the vertex attributes pointer for the shader
        gl::VertexAttribPointer(0, 2, gl::FLOAT, gl::FALSE, 0, std::ptr::null());

        // Draw the line using the defined vertices
        gl::DrawArrays(gl::LINE_STRIP, 0, vertices.len() as i32 / 2);
    }
}

fn render_pause_icon() {
    // Specify the points of the rectangle (x, y, z coords)
    let vertices: [f32; 24] = [
        // right rectangle of the pause icon
        0.94, 0.80, 0.0, // top right
        0.94, 0.95, 0.0, // bottom right
        0.95, 0.95, 0.0, // bottom left
        0.95, 0.80, 0.0, // top left
        // left rectangle of the pause icon
        0.92, 0.80, 0.0, // top right
        0.92, 0.95, 0.0, // bottom right
        0.93, 0.95, 0.0, // bottom left
        0.93, 0.80, 0.0, // top left
    ];

    // Specify the indices for drawing
    let indices: [u32; 12] = [
        0, 1, 3, // first triangle
        1, 2, 3, // second triangle
        4, 5, 7, // third triangle
        5, 6, 7, // fourth triangle
    ];

    unsafe {
        // Create a vertex array object
        let mut vao = 0;
        gl::GenVertexArrays(1, &mut vao);
        gl::BindVertexArray(vao);

        // Create a vertex buffer object
        let mut vbo = 0;
        gl::GenBuffers(1, &mut vbo);
        gl::BindBuffer(gl::ARRAY_BUFFER, vbo);
        gl::BufferData(
            gl::ARRAY_BUFFER,
            (vertices.len() * size_of::<gl::types::GLfloat>()) as gl::types::GLsizeiptr,
            &vertices[0] as *const f32 as *const gl::types::GLvoid,
            gl::STATIC_DRAW,
        );

        // Create an element buffer object
        let mut ebo = 0;
        gl::GenBuffers(1, &mut ebo);
        gl::BindBuffer(gl::ELEMENT_ARRAY_BUFFER, ebo);
        gl::BufferData(
            gl::ELEMENT_ARRAY_BUFFER,
            (indices.len() * size_of::<gl::types::GLuint>()) as gl::types::GLsizeiptr,
            &indices[0] as *const u32 as *const gl::types::GLvoid,
            gl::STATIC_DRAW,
        );

        // Specify the layout of the vertex data
        gl::VertexAttribPointer(
            0,
            3,
            gl::FLOAT,
            gl::FALSE,
            3 * size_of::<gl::types::GLfloat>() as gl::types::GLsizei,
            std::ptr::null(),
        );
        gl::EnableVertexAttribArray(0);

        // Draw the rectangle
        gl::DrawElements(
            gl::TRIANGLES,
            indices.len() as i32,
            gl::UNSIGNED_INT,
            std::ptr::null(),
        );
    }
}
