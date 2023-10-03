use std::collections::VecDeque;
use std::ffi::CString;
use std::mem::size_of;
use std::sync::{Arc, Mutex};

use glutin::display::Display;
use glutin::prelude::*;

use crate::notes::MAX_VOLUME;

/// Small helper to create (and set defaults) for uniforms
enum UniformDefault {
    F32(f32),
    Int(i32),
}

impl UniformDefault {
    pub fn create(self, program: u32, name: &str) -> i32 {
        let c_str = CString::new(name).unwrap();
        unsafe {
            let location = gl::GetUniformLocation(program, c_str.as_ptr());
            match self {
                UniformDefault::F32(value) => gl::Uniform1f(location, value),
                UniformDefault::Int(value) => gl::Uniform1i(location, value),
            }

            location
        }
    }
}

pub struct Renderer {
    audio_data: Arc<Mutex<VecDeque<f32>>>,
    audio_data_len: f32,

    u_audio_data_len: i32,
    u_is_drawing_pause_icon: i32,

    // (vao, vbo)
    vao_wave: (u32, u32),
    // (vao, count)
    vao_pause_icon: (u32, i32),

    vertices: Vec<f32>,
}

impl Renderer {
    pub fn new(gl_display: &Display, audio_data: Arc<Mutex<VecDeque<f32>>>) -> Renderer {
        unsafe {
            // provide loader to link gl function pointers to the display
            gl::load_with(|symbol| {
                let symbol = CString::new(symbol).unwrap();
                gl_display.get_proc_address(symbol.as_c_str()).cast()
            });

            // compile shaders
            let vertex_source = CString::new(include_str!("./vertex.vert")).unwrap();
            let fragment_source = CString::new(include_str!("./fragment.glsl")).unwrap();

            let vertex_shader = gl::CreateShader(gl::VERTEX_SHADER);
            gl::ShaderSource(vertex_shader, 1, &vertex_source.as_ptr(), std::ptr::null());
            gl::CompileShader(vertex_shader);

            let fragment_shader = gl::CreateShader(gl::FRAGMENT_SHADER);
            gl::ShaderSource(
                fragment_shader,
                1,
                &fragment_source.as_ptr(),
                std::ptr::null(),
            );
            gl::CompileShader(fragment_shader);

            // link shaders into a program
            let shader_program = gl::CreateProgram();
            gl::AttachShader(shader_program, vertex_shader);
            gl::AttachShader(shader_program, fragment_shader);
            gl::LinkProgram(shader_program);

            // we can delete the shaders now, since they're linked into the program now
            gl::DeleteShader(vertex_shader);
            gl::DeleteShader(fragment_shader);

            // set the program we just created to be the active one
            gl::UseProgram(shader_program);

            /*
             * Setup VAO for rendering wave
             */

            let vao_wave = {
                let mut vao: gl::types::GLuint = 0;
                gl::GenVertexArrays(1, &mut vao);
                gl::BindVertexArray(vao);

                let mut vbo: gl::types::GLuint = 0;
                gl::GenBuffers(1, &mut vbo);
                gl::BindBuffer(gl::ARRAY_BUFFER, vbo);
                gl::VertexAttribPointer(0, 2, gl::FLOAT, gl::FALSE, 0, std::ptr::null());
                gl::EnableVertexAttribArray(0);

                (vao, vbo)
            };

            /*
             * Setup VAO for rendering the pause icon
             */
            let vao_pause_icon = {
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

                (vao, indices.len() as i32)
            };

            // create uniforms
            UniformDefault::F32(MAX_VOLUME).create(shader_program, "audio_max_volume");
            let u_audio_data_len =
                UniformDefault::F32(0.0).create(shader_program, "audio_data_len");
            let u_is_drawing_pause_icon =
                UniformDefault::Int(0).create(shader_program, "is_drawing_pause_icon");

            Renderer {
                audio_data,
                audio_data_len: 0.0,

                u_audio_data_len,
                u_is_drawing_pause_icon,

                vao_wave,
                vao_pause_icon,

                vertices: vec![0.0],
            }
        }
    }

    pub fn draw(&mut self, paused: bool) {
        unsafe {
            // Set the clearing color to black (R=0, G=0, B=0) with full opacity (A=1.0)
            gl::ClearColor(0.0, 0.0, 0.0, 1.0);
            // Clear the color buffer with the set color
            gl::Clear(gl::COLOR_BUFFER_BIT);

            if paused {
                self.render_pause_icon();
            } else {
                // fetch and prepare audio data to be sent to gl
                let audio_data = self.audio_data.lock().unwrap();
                self.audio_data_len = audio_data.len() as f32;
                self.vertices = audio_data
                    .iter()
                    .enumerate()
                    // we send the index and the audio value (y) over to the shader
                    .flat_map(|(idx, y)| vec![idx as f32, *y])
                    .collect();
            };

            self.render_wave();
        }
    }

    fn render_wave(&self) {
        unsafe {
            let (vao, vbo) = self.vao_wave;
            gl::Uniform1f(self.u_audio_data_len, self.audio_data_len);
            gl::BindVertexArray(vao);
            gl::BindBuffer(gl::ARRAY_BUFFER, vbo);
            gl::BufferData(
                gl::ARRAY_BUFFER,
                (self.vertices.len() * size_of::<f32>()) as gl::types::GLsizeiptr,
                &self.vertices[0] as *const f32 as *const gl::types::GLvoid,
                gl::STATIC_DRAW,
            );
            gl::DrawArrays(gl::LINE_STRIP, 0, self.vertices.len() as i32 / 2);
        }
    }

    fn render_pause_icon(&self) {
        unsafe {
            let (vao, count) = self.vao_pause_icon;
            gl::Uniform1i(self.u_is_drawing_pause_icon, 1);
            gl::BindVertexArray(vao);
            gl::DrawElements(gl::TRIANGLES, count, gl::UNSIGNED_INT, std::ptr::null());
            gl::Uniform1i(self.u_is_drawing_pause_icon, 0);
        }
    }
}
