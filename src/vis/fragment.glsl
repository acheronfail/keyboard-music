#version 330 core

out vec4 color;
uniform bool is_drawing_pause_icon;

void main() {
    if (is_drawing_pause_icon) {
        color = vec4(1.0, 0.0, 0.0, 1.0);
    } else {
        color = vec4(1.0, 1.0, 1.0, 1.0);
    }
}