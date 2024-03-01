#version 330
//author https://github.com/autergame

out vec4 FragColor;

void main()
{       
    vec2 cxy = 2.0 * gl_PointCoord - 1.0;
    float r = dot(cxy, cxy);
	float delta = fwidth(r);
	float alpha = 1.0 - smoothstep(1.0 - delta, 1.0 + delta, r);

	FragColor = vec4(1.0, 0.0, 0.0, alpha);
}