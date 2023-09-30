#version 330 core

layout (location = 0) in vec3 data;
uniform float audio_data_len;
uniform float audio_max_volume;
uniform bool rendering_wave;

const float max_y = 0.8;

void main() {
    if (rendering_wave) {
        // normalise x position between -1.0 and 1.0
        float x = (data.x / audio_data_len) * 2.0 - 1.0;
        // also normalise y, but not to the entire height (make it easier to see when the wave is clipping)
        // clamp y value to demonstrate clipped values
        float y = clamp((data.y / audio_max_volume) * max_y, -max_y, max_y);
        gl_Position = vec4(x, y, 1.0, 1.0);
    } else {
        gl_Position = vec4(data, 1.0);
    }
}
