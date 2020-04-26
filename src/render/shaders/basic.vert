#version 140

in vec4 pos;
in vec4 color;

out vec4 vert_color;

uniform bool use_override_color;
uniform vec4 override_color;
uniform vec4 view_vector;
uniform mat4 perspective_matrix;

void main() {
    gl_Position = perspective_matrix * (view_vector + pos);

    if (use_override_color) {
        vert_color = override_color;
        // Move point *slightly* closer to the camera to avoid Z-fighting.
        gl_Position.z -= 0.02;
    } else {
        vert_color = color;
    }
}
