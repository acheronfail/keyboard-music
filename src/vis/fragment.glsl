#version 330 core

out vec4 color;
uniform bool rendering_wave;

void main() {
    if (rendering_wave) {
        color = vec4(1.0, 1.0, 1.0, 1.0);
    } else {
        color = vec4(1.0, 0.0, 0.0, 1.0);
    }
}