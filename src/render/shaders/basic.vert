#version 140

in vec4 pos;
in vec4 color;

out vec4 vert_color;

uniform bool lines;
uniform mat4 model_matrix;
uniform vec4 view_translation;
uniform mat4 perspective_matrix;

// uniform vec3 light;
// uniform float maxLight;
// uniform float minLight;

void main() {
    // float light_amt = (dot(normalize(light), normalize(cross())) + 1) / 2
    gl_Position = perspective_matrix * (view_translation + (model_matrix * pos));
    if (lines) {
        // Set color to black.
        vert_color = vec4(0.0, 0.0, 0.0, 1.0);
        // Move point *slightly* closer to the camera to avoid Z-fighting.
        gl_Position.z -= 0.01;
    } else {
        vert_color = color;
    }
}
