#version 450
#extension GL_GOOGLE_include_directive : require
#include "base_frag.glsl"
#include "base_uniforms.glsl"
#include "image_tools.glsl"
#include "tonemapping.glsl"
#include "film_grain.glsl"
#include "toon.glsl"
#include "pixelate.glsl"

// Base Color Texture
layout(set = 0, binding = 0) uniform texture2D t_diffuse;
layout(set = 0, binding = 1) uniform sampler s_diffuse;

// HDR Texture (Used for bloom)
layout(set = 1, binding = 0) uniform texture2D hdr_t_diffuse;
layout(set = 1, binding = 1) uniform sampler hdr_s_diffuse;

void main()
{
    const float gamma = 2.2f;
    const float exposure = 0.075f;

    vec3 hdrColor = texture(sampler2D(t_diffuse, s_diffuse), v_tex_coords).rgb;
    hdrColor = toonify(hdrColor, t_diffuse, s_diffuse, v_tex_coords);


    vec3 bloomColor = texture(sampler2D(hdr_t_diffuse, hdr_s_diffuse), v_tex_coords).rgb;

    /* Image Effects */

    // Apply the bloom hdr effect by additive

    // Color correct the bloom
    vec4 result = vec4(vec3(1.0) - exp(-hdrColor * exposure), 1.0);


    // Effects

    result *= vignette(v_tex_coords, .3, 0.3);
    result += film_grain(0.0005, v_tex_coords);

    /* Image Correction */

    

    result.rgb = adjustContrast(result.rgb, 0.025);
    
    result.rgb = adjustSaturation(result.rgb, 0.5);

    result.rgb = adjustExposure(result.rgb, 0.5);

    result.rgb *= chromaticAberration(t_diffuse, s_diffuse, v_tex_coords, 0.125);


    // Bloom 

    result.rgb += bloomColor;

    // Color Correct
    result.rgb = acesFilm(result.rgb);

    // Gamma Correction for HDR to LDR
    result = pow(result, vec4(1.0 / gamma));

    f_color = result;
}  