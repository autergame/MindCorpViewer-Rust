#version 330
//author https://github.com/autergame

in vec3 Color;

void main()
{       
    gl_FragColor = vec4(Color, 1.0);
}