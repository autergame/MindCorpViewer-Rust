#version 330
//author https://github.com/autergame

layout (location = 0) in vec3 Positions;
layout (location = 1) in vec2 UVs;
layout (location = 2) in uvec4 BoneIndices;
layout (location = 3) in vec4 BoneWeights;

out vec2 UV;

uniform mat4 MVP;
uniform int UseBone;

layout (std140) uniform BonesTransformsBlock {
    mat4 BonesTransforms[256];
};

void main()
{
    UV = UVs;

	if (UseBone == 1) {
		mat4 BoneTransform = BonesTransforms[BoneIndices[0]] * BoneWeights[0];
		BoneTransform     += BonesTransforms[BoneIndices[1]] * BoneWeights[1];
		BoneTransform     += BonesTransforms[BoneIndices[2]] * BoneWeights[2];
		BoneTransform     += BonesTransforms[BoneIndices[3]] * BoneWeights[3];

		gl_Position = MVP * BoneTransform * vec4(Positions, 1.0);
	} else {
		gl_Position = MVP * vec4(Positions, 1.0);
	}
}