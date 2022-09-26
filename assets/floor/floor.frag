#version 330
//author https://github.com/autergame

in vec2 UV;

uniform sampler2D Diffuse;

void main()
{
	gl_FragColor = texture(Diffuse, UV);
}