// Single-pass wireframe rendering
// http://www2.imm.dtu.dk/pubdb/edoc/imm4884.pdf
// https://github.com/arefin86/arefin86.github.io/blob/master/wireframe.html

#version 140

in vec4 v0;
in vec4 v1;
in vec4 v2;
in vec4 fill_color;
in vec4 wire_color;
in vec3 line_mask;

out vec4 vert_fill_color;
out vec4 vert_wire_color;
noperspective out vec3 dist;

uniform vec2 target_size;
uniform mat4 transform;

void main() {
    vert_fill_color = fill_color;
    vert_wire_color = wire_color;

    vec4 p0 = transform * v0;
    vec4 p1 = transform * v1;
    vec4 p2 = transform * v2;
    gl_Position = p0;

    // Compute pixel coordinates.
    vec2 a = p0.xy / p0.w * target_size / 2.0;
    vec2 b = p1.xy / p1.w * target_size / 2.0;
    vec2 c = p2.xy / p2.w * target_size / 2.0;

    // Compute area of triangle using determinant.
    vec2 ab = (b - a);
    vec2 ac = (c - a);
    vec2 bc = (b - c);
    float area = abs(ab.x * bc.y - ab.y * bc.x);
    // Compute "height" of triangle; i.e. distance from point a to the line bc.
    float h = area / length(bc);

    dist = vec3(100.0, 100.0, 100.0) - 100.0 * line_mask;
    dist[gl_VertexID % 3] += h;
}
