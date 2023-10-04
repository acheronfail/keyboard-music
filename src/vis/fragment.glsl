#version 330 core

out vec4 color;
uniform bool is_drawing_wave;

void main() {
    if (is_drawing_wave) {
        color = vec4(1.0, 1.0, 1.0, 1.0);
    } else {
        color = vec4(1.0, 0.0, 0.0, 1.0);
    }
}