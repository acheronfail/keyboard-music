use std::collections::VecDeque;
use std::ffi::CString;
use std::mem::size_of;
use std::sync::{Arc, Mutex};

use glutin::display::Display;
use glutin::prelude::*;

use crate::notes::MAX_VOLUME;

pub struct Renderer {
    audio_data: Arc<Mutex<VecDeque<f32>>>,
    audio_data_len: f32,
    audio_max_volume: f32,

    u_audio_data_len: i32,
    u_audio_max_volume: i32,
    u_is_drawing_pause_icon: i32,

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

            // helper to get uniform locations
            let create_uniform = |name| {
                let c_str = CString::new(name).unwrap();
                gl::GetUniformLocation(shader_program, c_str.as_ptr())
            };

            Renderer {
                audio_data,
                audio_data_len: 0.0,
                audio_max_volume: MAX_VOLUME,

                u_audio_data_len: create_uniform("audio_data_len"),
                u_audio_max_volume: create_uniform("audio_max_volume"),
                u_is_drawing_pause_icon: create_uniform("is_drawing_pause_icon"),

                vertices: vec![],
            }
        }
    }

    pub fn draw(&mut self, paused: bool) {
        unsafe {
            // Set the clearing color to black (R=0, G=0, B=0) with full opacity (A=1.0)
            gl::ClearColor(0.0, 0.0, 0.0, 1.0);
            // Clear the color buffer with the set color
            gl::Clear(gl::COLOR_BUFFER_BIT);

            // setup uniforms
            gl::Uniform1f(self.u_audio_data_len, self.audio_data_len);
            gl::Uniform1f(self.u_audio_max_volume, self.audio_max_volume);

            // fetch and prepare data to be sent to gl
            if paused {
                self.render_pause_icon();
            } else {
                let audio_data = self.audio_data.lock().unwrap();
                self.audio_data_len = audio_data.len() as f32;
                self.vertices = audio_data
                    .iter()
                    .enumerate()
                    // we send the index and the audio value (y) over to the shader
                    .flat_map(|(idx, y)| vec![idx as f32, *y])
                    .collect();
            };

            // Prepare variables for the Vertex Buffer Object (vbo) and Vertex Array Object (vao)
            let mut vbo: gl::types::GLuint = 0;
            let mut vao: gl::types::GLuint = 0;

            // Generate the buffer for vertices and bind it to the vbo
            gl::GenBuffers(1, &mut vbo);
            gl::BindBuffer(gl::ARRAY_BUFFER, vbo);

            // Send the vertex data to the GPU, specifying it's a static draw (not expected to change)
            gl::BufferData(
                gl::ARRAY_BUFFER,
                (self.vertices.len() * size_of::<f32>()) as gl::types::GLsizeiptr,
                &self.vertices[0] as *const f32 as *const gl::types::GLvoid,
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
            gl::DrawArrays(gl::LINE_STRIP, 0, self.vertices.len() as i32 / 2);
        }
    }

    fn render_pause_icon(&self) {
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
            gl::Uniform1i(self.u_is_drawing_pause_icon, 1);

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

            gl::Uniform1i(self.u_is_drawing_pause_icon, 0);
        }
    }
}
