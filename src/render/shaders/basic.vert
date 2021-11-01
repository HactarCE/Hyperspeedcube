#version 140

in vec4 pos;
in vec4 color;

out vec4 vert_color;

void main() {
    gl_Position = pos;
    vert_color = color;
}
