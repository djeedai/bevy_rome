struct View {
    view_proj: mat4x4<f32>;
    world_position: vec3<f32>;
};

[[group(0), binding(0)]]
var<uniform> view: View;

struct VertexOutput {
    [[builtin(position)]] position: vec4<f32>;
    [[location(0)]] color: vec4<f32>;
#ifdef TEXTURED
    [[location(1)]] uv: vec2<f32>;
#endif
};

[[stage(vertex)]]
fn vertex(
    [[location(0)]] vertex_position: vec3<f32>,
    [[location(1)]] vertex_color: vec4<f32>,
#ifdef TEXTURED
    [[location(2)]] vertex_uv: vec2<f32>,
#endif
) -> VertexOutput {
    var out: VertexOutput;
    out.position = view.view_proj * vec4<f32>(vertex_position, 1.0);
    out.color = vertex_color;
#ifdef TEXTURED
    out.uv = vertex_uv;
#endif
    return out;
}

#ifdef TEXTURED
[[group(1), binding(0)]]
var quad_texture: texture_2d<f32>;
[[group(1), binding(1)]]
var quad_sampler: sampler;
#endif

[[stage(fragment)]]
fn fragment(in: VertexOutput) -> [[location(0)]] vec4<f32> {
    var color = in.color;
#ifdef TEXTURED
    color *= textureSample(quad_texture, quad_sampler, in.uv);
#endif
    return color;
}
