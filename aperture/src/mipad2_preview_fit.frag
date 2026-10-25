/* SPDX-License-Identifier: MIT */
#version 100
#ifdef GL_ES
precision highp float;
#endif
varying vec2 v_texcoord;
uniform sampler2D tex;
/* GStreamer's width/height uniforms describe OUTPUT dimensions. */
uniform float width;
uniform float height;
/* Source display aspect ratio includes its pixel aspect ratio. */
uniform float source_dar;
void main() {
    float target_dar = width / height;
    vec2 fit = vec2(min(1.0, source_dar / target_dar),
                    min(1.0, target_dar / source_dar));
    vec2 uv = (v_texcoord - vec2(0.5)) / fit + vec2(0.5);
    if (uv.x < 0.0 || uv.x > 1.0 || uv.y < 0.0 || uv.y > 1.0)
        gl_FragColor = vec4(0.0, 0.0, 0.0, 1.0);
    else
        gl_FragColor = texture2D(tex, uv);
}
