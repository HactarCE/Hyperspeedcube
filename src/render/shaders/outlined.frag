// Single-pass wireframe rendering
// http://www2.imm.dtu.dk/pubdb/edoc/imm4884.pdf
// https://github.com/arefin86/arefin86.github.io/blob/master/wireframe.html

#version 140

in vec4 vert_fill_color;
in vec4 vert_wire_color;
noperspective in vec3 dist;

out vec4 color;

uniform float wire_width;

const float GAMMA = 2.2;

vec4 srgb_to_linear(vec4 color) {
  return vec4(pow(color.rgb, vec3(GAMMA, GAMMA, GAMMA)), color.a);
}

vec4 linear_to_srgb(vec4 color) {
  return vec4(pow(color.rgb, 1.0 / vec3(GAMMA, GAMMA, GAMMA)), color.a);
}

void main() {
    // Compute the shortest distance to any edge.
    float d = min(dist.x, min(dist.y, dist.z));

    d = max(0, d - wire_width + 1.0);

    // This is the equation they use in the original paper, although I don't
    // know its derivation or where it's even written.
    float a = exp2(-2.0 * d * d);

    color = mix(vert_fill_color, vert_wire_color, a);

    // Unpremultiply alpha.
    if (color.a != 0.0) {
      color.rgb /= color.a;
    }
}
