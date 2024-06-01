// FIXME : #import from Bevy itself...
struct ColorGrading {
    exposure: f32,
    gamma: f32,
    pre_saturation: f32,
    post_saturation: f32,
}

// FIXME : #import from Bevy itself...
struct View {
    view_proj: mat4x4<f32>,
    unjittered_view_proj: mat4x4<f32>,
    inverse_view_proj: mat4x4<f32>,
    view: mat4x4<f32>,
    inverse_view: mat4x4<f32>,
    projection: mat4x4<f32>,
    inverse_projection: mat4x4<f32>,
    world_position: vec3<f32>,
    exposure: f32,
    // viewport(x_origin, y_origin, width, height)
    viewport: vec4<f32>,
    frustum: array<vec4<f32>, 6>,
    color_grading: ColorGrading,
    mip_bias: f32,
    render_layers: u32,
};

/// Serialized primitives buffer.
struct Primitives {
    elems: array<f32>,
};

@group(0) @binding(0)
var<uniform> view: View;

@group(1) @binding(0)
var<storage, read> primitives: Primitives;

#ifdef TEXTURED
@group(2) @binding(0)
var quad_texture: texture_2d<f32>;
@group(2) @binding(1)
var quad_sampler: sampler;
#endif

struct VertexOutput {
    @builtin(position) position: vec4<f32>,
    @location(0) color: vec4<f32>,
    @location(1) radii: vec2<f32>,
#ifdef TEXTURED
    @location(2) uv: vec2<f32>,
#endif
};

struct Primitive {
    /// Offset into the primitive buffer.
    offset: u32,
    /// Kind of primitive.
    kind: u32,
    /// Rectangle corner, in [0:1].
    corner: vec2<f32>,
    /// Texture index.
    tex_index: u32,
};

struct Rect {
    pos: vec2<f32>,
    size: vec2<f32>,
    color: vec4<f32>,
#ifdef TEXTURED
    uv_pos: vec2<f32>,
    uv_size: vec2<f32>,
#endif
};

struct QPie {
    origin: vec2<f32>,
    radii: vec2<f32>,
    color: vec4<f32>,
};

struct Line {
    /// Line origin.
    origin: vec2<f32>,
    /// Line direction, of length the segment length.
    dir: vec2<f32>,
    /// Normal vector (normalized).
    normal: vec2<f32>,
    /// Line color.
    color: vec4<f32>,
    /// Line thickness, in the direction of the normal.
    thickness: f32,
};

fn unpack_index(vertex_index: u32) -> Primitive {
    var p: Primitive;
    p.offset = (vertex_index & 0x00FFFFFFu);
    let cx = (vertex_index & 0x01000000u) >> 24u;
    let cy = (vertex_index & 0x02000000u) >> 25u;
    p.corner = vec2<f32>(f32(cx), f32(cy));
    p.kind = (vertex_index & 0x1C000000u) >> 26u;
    p.tex_index = (vertex_index & 0xE0000000u) >> 29u;
    return p;
}

fn load_rect(offset: u32) -> Rect {
    let x = primitives.elems[offset];
    let y = primitives.elems[offset + 1u];
    let w = primitives.elems[offset + 2u];
    let h = primitives.elems[offset + 3u];
    let c = primitives.elems[offset + 4u];
#ifdef TEXTURED
    let uv_x = primitives.elems[offset + 5u];
    let uv_y = primitives.elems[offset + 6u];
    let uv_w = primitives.elems[offset + 7u];
    let uv_h = primitives.elems[offset + 8u];
#endif
    var rect: Rect;
    rect.pos = vec2<f32>(x, y);
    rect.size = vec2<f32>(w, h);
    let uc: u32 = bitcast<u32>(c);
    rect.color = unpack4x8unorm(uc);
#ifdef TEXTURED
    rect.uv_pos = vec2<f32>(uv_x, uv_y);
    rect.uv_size = vec2<f32>(uv_w, uv_h);
#endif
    return rect;
}

fn load_line(offset: u32) -> Line {
    let p0x = primitives.elems[offset];
    let p0y = primitives.elems[offset + 1u];
    let p1x = primitives.elems[offset + 2u];
    let p1y = primitives.elems[offset + 3u];
    let c = primitives.elems[offset + 4u];
    let t = primitives.elems[offset + 5u];
    var lin: Line;
    let p0 = vec2<f32>(p0x, p0y);
    let p1 = vec2<f32>(p1x, p1y);
    lin.origin = p0;
    lin.dir = p1 - p0;
    lin.normal = normalize(vec2<f32>(-lin.dir.y, lin.dir.x));
    let uc: u32 = bitcast<u32>(c);
    lin.color = unpack4x8unorm(uc);
    lin.thickness = t;
    return lin;
}

fn load_qpie(offset: u32) -> QPie {
    let x = primitives.elems[offset];
    let y = primitives.elems[offset + 1u];
    let rx = primitives.elems[offset + 2u];
    let ry = primitives.elems[offset + 3u];
    let c = primitives.elems[offset + 4u];
    var qpie: QPie;
    qpie.origin = vec2<f32>(x, y);
    qpie.radii = vec2<f32>(rx, ry);
    let uc: u32 = bitcast<u32>(c);
    qpie.color = unpack4x8unorm(uc);
    return qpie;
}

@vertex
fn vertex(
    @builtin(vertex_index) vertex_index: u32,
) -> VertexOutput {
    let prim = unpack_index(vertex_index);
    var out: VertexOutput;
    out.radii = vec2<f32>(0.);
    var vertex_position: vec2<f32>;
    if (prim.kind <= 1u) { // RECT or GLYPH
        let rect = load_rect(prim.offset);
        vertex_position = rect.pos + rect.size * prim.corner;
        out.color = rect.color;
#ifdef TEXTURED
        out.uv = rect.uv_pos + rect.uv_size * prim.corner;
#endif
    } else if (prim.kind == 2u) { // LINE
        let lin = load_line(prim.offset);
        vertex_position = lin.origin + lin.dir * prim.corner.x + lin.normal * ((prim.corner.y - 0.5) * lin.thickness);
        out.color = lin.color;
    } else if (prim.kind == 3u) { // QUARTER PIE
        let qpie = load_qpie(prim.offset);
        vertex_position = qpie.origin + qpie.radii * prim.corner;
        out.radii = prim.corner; //abs(qpie.radii);
        //out.color = vec4<f32>(prim.corner, 0., 1.); // TEMP - for debugging //qpie.color;
        out.color = qpie.color;
    }
    out.position = view.view_proj * vec4<f32>(vertex_position, 0.0, 1.0);
    return out;
}

@fragment
fn fragment(in: VertexOutput) -> @location(0) vec4<f32> {
    var color = in.color;
#ifdef TEXTURED
    let rgba = textureSample(quad_texture, quad_sampler, in.uv);
    color = color * rgba;
#endif
    if (in.radii.x > 0.) {
        let eps = 0.1;
        let r2 = 1. - dot(in.radii, in.radii);
        //color = vec4<f32>(r2, r2, r2, 1.);
        let r = smoothstep(0., 1., (r2 + eps) / (2. * eps));
        //color = vec4<f32>(r, r, r, 1.);
        color.a = color.a * r;
    }
    return color;
}
