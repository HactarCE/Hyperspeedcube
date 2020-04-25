#version 140

in vec3 pos;
in vec4 color;

out vec4 tri_color;

uniform mat4 matrix;
// uniform vec3 light;
// uniform float maxLight;
// uniform float minLight;

void main() {
    // float light_amt = (dot(normalize(light), normalize(cross())) + 1) / 2
    gl_Position = matrix * vec4(pos, 1.0);
    tri_color = color;
}
