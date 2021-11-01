#version 140

in vec4 pos;
in vec4 color;

out vec4 vert_color;

uniform bool use_override_color;
uniform vec4 override_color;

const float EPSILON = 0.01;

void main() {
    gl_Position = pos;

    if (use_override_color) {
        vert_color = override_color;
        vert_color.a = color.a;
        // Move point *slightly* closer to the camera to avoid Z-fighting.
        // gl_Position.z -= EPSILON * (gl_Position.z + 1.0);
    } else {
        vert_color = color;
    }
}
