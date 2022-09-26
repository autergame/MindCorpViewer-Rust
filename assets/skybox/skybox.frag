#version 330
//author https://github.com/autergame

in vec3 UV;

uniform samplerCube Skybox;

void main()
{    
    gl_FragColor = texture(Skybox, UV);
}