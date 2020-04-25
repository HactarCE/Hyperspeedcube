#version 140

in vec4 pos;
in vec4 color;

out vec4 tri_color;

uniform mat4 model_matrix;
uniform vec4 view_translation;
uniform mat4 perspective_matrix;

// uniform vec3 light;
// uniform float maxLight;
// uniform float minLight;

void main() {
    // float light_amt = (dot(normalize(light), normalize(cross())) + 1) / 2
    gl_Position = perspective_matrix * (view_translation + (model_matrix * pos));
    tri_color = color;
}
