struct View {
    view_proj: mat4x4<f32>;
    world_position: vec3<f32>;
};

struct Primitives {
    elems: array<f32>;
};

[[group(0), binding(0)]]
var<uniform> view: View;

[[group(1), binding(0)]]
var<storage> primitives: Primitives;

#ifdef TEXTURED
[[group(2), binding(0)]]
var quad_texture: texture_2d<f32>;
[[group(2), binding(1)]]
var quad_sampler: sampler;
#endif

struct VertexOutput {
    [[builtin(position)]] position: vec4<f32>;
    [[location(0)]] color: vec4<f32>;
#ifdef TEXTURED
    [[location(1)]] uv: vec2<f32>;
#endif
};

struct Primitive {
    offset: u32;
    kind: u32;
    corner: vec2<f32>;
};

struct Rect {
    pos: vec2<f32>;
    size: vec2<f32>;
    color: vec4<f32>;
};

fn unpack_index(vertex_index: u32) -> Primitive {
    var p: Primitive;
    p.offset = (vertex_index & 0x00FFFFFFu);
    let cx = (vertex_index & 0x01000000u) >> 24u;
    let cy = (vertex_index & 0x02000000u) >> 25u;
    p.corner = vec2<f32>(f32(cx), f32(cy));
    p.kind = (vertex_index & 0xF6000000u) >> 26u;
    return p;
}

fn load_rect(offset: u32) -> Rect {
    let x = primitives.elems[offset];
    let y = primitives.elems[offset + 1u];
    let w = primitives.elems[offset + 2u];
    let h = primitives.elems[offset + 3u];
    let c = primitives.elems[offset + 4u];
    var rect: Rect;
    rect.pos = vec2<f32>(x, y);
    rect.size = vec2<f32>(w, h);
    let uc: u32 = bitcast<u32>(c);
    rect.color = unpack4x8unorm(uc);
    return rect;
}

[[stage(vertex)]]
fn vertex(
    [[builtin(vertex_index)]] vertex_index: u32,
) -> VertexOutput {
    var prim = unpack_index(vertex_index);
    var out: VertexOutput;
    var vertex_position: vec2<f32>;
    var vertex_color: vec4<f32>;
    // //if (prim.kind == 0u) // RECT
    // {
    //     //let rect = load_rect(prim.offset);
    //     var rect: Rect;
    //     rect.pos = vec2<f32>(0., 0.);
    //     rect.size = vec2<f32>(10., 20.);
    //     rect.color = vec4<f32>(1., 1., 1., 1.);

    //     // TEMP
    //     prim.corner = vec2<f32>(f32(vertex_index & 1u), 1. - f32((vertex_index & 2u) >> 1u));

    //     vertex_position = rect.pos + rect.size * prim.corner;
    //     vertex_color = rect.color;
    // }
    if (vertex_index == 0u)
    {
        vertex_position = vec2<f32>(-100., -100.);
    }
    else if (vertex_index == 1u)
    {
        vertex_position = vec2<f32>(100., -100.);
    }
    else if (vertex_index == 2u)
    {
        vertex_position = vec2<f32>(100., 100.);
    }
    else if (vertex_index == 3u)
    {
        vertex_position = vec2<f32>(-100., 100.);
    }
    vertex_color = vec4<f32>(1., 1., 1., 1.);
    out.position = view.view_proj * vec4<f32>(vertex_position, 0.0, 1.0);
    out.color = vertex_color;
#ifdef TEXTURED
    //out.uv = vertex_uv;
#endif
    return out;
}

[[stage(fragment)]]
fn fragment(in: VertexOutput) -> [[location(0)]] vec4<f32> {
    var color = in.color;
#ifdef TEXTURED
    color *= textureSample(quad_texture, quad_sampler, in.uv);
#endif
    return color;
}
