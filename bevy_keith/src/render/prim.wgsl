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
}

// Keep in sync with GpuPrimitiveKind
const PRIM_RECT: u32 = 0u;
const PRIM_GLYPH: u32 = 1u;
const PRIM_LINE: u32 = 2u;
const PRIM_QUARTER_PIE: u32 = 3u;

/// Serialized primitives buffer.
struct Primitives {
    elems: array<f32>,
}

/// Offset where the list of primitives for a tile starts,
/// and number of consecutive primitives for that tile.
struct OffsetAndCount {
    /// Offset into Tile::primitives[].
    offset: u32,
    /// Number of consecutive primitive indices in Tile::primitives[].
    count: u32,
}

struct Tiles {
    /// Packed index and kind of primitives.
    primitives: array<u32>,
}

@group(0) @binding(0)
var<uniform> view: View;

@group(1) @binding(0)
var<storage, read> primitives: Primitives;
@group(1) @binding(1)
var<storage, read> tiles: Tiles;
@group(1) @binding(2)
var<storage, read> offsets_and_counts: array<OffsetAndCount>;

@group(2) @binding(0)
var quad_texture: texture_2d<f32>;
@group(2) @binding(1)
var quad_sampler: sampler;

struct VertexOutput {
    @builtin(position) position: vec4<f32>,
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
}

struct Rect {
    /// Center point.
    center: vec2<f32>,
    /// Half extents.
    half_size: vec2<f32>,
    /// Corners radius.
    radius: f32,
    /// Color.
    color: vec4<f32>,
    /// Origin of UV coordinates.
    uv_origin: vec2<f32>,
    /// Scale of UV coordinates.
    uv_scale: vec2<f32>,
}

struct QPie {
    origin: vec2<f32>,
    radii: vec2<f32>,
    color: vec4<f32>,
}

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
}

const TILE_SIZE = vec2<f32>(8., 8.);

/// Get the total number of tiles in the buffer.
fn get_tile_count() -> u32 {
    // By design the offset and count buffer has one entry per tile
    return arrayLength(&offsets_and_counts);
}

fn get_tile_dim() -> vec2<u32> {
    let xy = ceil(view.viewport.zw / TILE_SIZE);
    return vec2<u32>(u32(xy.x), u32(xy.y));
}

struct IndexAndKind {
    index: u32,
    kind: u32,
    textured: bool,
}

fn unpack_index_and_kind(value: u32) -> IndexAndKind {
    let index = (value & 0x0FFFFFFFu);
    let kind = (value & 0x70000000u) >> 28u;
    let textured = (value & 0x80000000u) != 0u;
    return IndexAndKind(index, kind, textured);
}

fn load_rect(offset: u32, textured: bool) -> Rect {
    let x = primitives.elems[offset];
    let y = primitives.elems[offset + 1u];
    let hw = primitives.elems[offset + 2u];
    let hh = primitives.elems[offset + 3u];
    let r = primitives.elems[offset + 4u];
    let c = primitives.elems[offset + 5u];
    var rect: Rect;
    rect.center = vec2<f32>(x, y);
    rect.half_size = vec2<f32>(hw, hh);
    rect.radius = r;
    let uc: u32 = bitcast<u32>(c);
    rect.color = unpack4x8unorm(uc);
    if (textured) {
        let uv_x = primitives.elems[offset + 6u];
        let uv_y = primitives.elems[offset + 7u];
        let uv_sx = primitives.elems[offset + 8u];
        let uv_sy = primitives.elems[offset + 9u];
        rect.uv_origin = vec2<f32>(uv_x, uv_y);
        rect.uv_scale = vec2<f32>(uv_sx, uv_sy);
    }
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

fn get_vertex_pos(vertex_index: u32) -> vec2<f32> {
    switch vertex_index {
        case 0u { return vec2<f32>(-1., -1.); }
        case 1u { return vec2<f32>(3., -1.); }
        case 2u { return vec2<f32>(-1., 3.); }
        default { return vec2<f32>(1e38, 1e38); }
    }
}

fn sdf_rect(base: u32, canvas_pos: vec2<f32>, textured: bool) -> vec4<f32> {
    let rect = load_rect(base, textured);
    let delta = abs(canvas_pos - rect.center) - rect.half_size + rect.radius;
    let dist = length(max(delta, vec2<f32>(0))) + max(min(delta.x, 0.), min(delta.y, 0.)) - rect.radius;
    let ratio = dist + 0.5; // pixel center is at 0.5 from actual border
    let alpha = smoothstep(rect.color.a, 0., ratio);
    var color = rect.color.rgb;
    if (textured) {
        let uv = (canvas_pos - rect.center) * rect.uv_scale + rect.uv_origin;
        color *= textureSample(quad_texture, quad_sampler, uv).rgb;
    }
    return vec4<f32>(color, alpha);
}

fn sdf_line(base: u32, canvas_pos: vec2<f32>) -> vec4<f32> {
    let line = load_line(base);
    let d = normalize(line.dir);
    let center = line.origin + line.dir / 2.;
    let delta = canvas_pos - center;
    let rd = mat2x2<f32>(d.x, -d.y, d.y, d.x) * delta;
    let delta2 = abs(rd) - vec2<f32>(length(line.dir), line.thickness) * 0.5;
    let dist = length(max(delta2, vec2<f32>(0))) + max(min(delta2.x, 0.), min(delta2.y, 0.));
    let ratio = dist + 0.5; // pixel center is at 0.5 from actual border
    let alpha = smoothstep(line.color.a, 0., ratio);
    return vec4<f32>(line.color.rgb, alpha);
}

@vertex
fn vertex(
    @builtin(vertex_index) vertex_index: u32,
) -> VertexOutput {
    var out: VertexOutput;
    out.position = vec4<f32>(get_vertex_pos(vertex_index), 0.0, 1.0);
    return out;
}

@fragment
fn fragment(in: VertexOutput) -> @location(0) vec4<f32> {
    // Find the tile this fragment is part of
    let tile_pos = floor(in.position.xy / TILE_SIZE);
    let tile_dim = get_tile_dim();
    let tile_index = u32(tile_pos.y) * tile_dim.x + u32(tile_pos.x);

    let canvas_pos = in.position.xy - view.viewport.zw / 2.;
    var color = vec4<f32>();

    // Loop over all primitives for that tile, and accumulate color
    var prim_offset = offsets_and_counts[tile_index].offset;
    let prim_count = offsets_and_counts[tile_index].count;
    for (var i = prim_offset; i < prim_offset + prim_count; i += 1u) {
        let index_and_kind = unpack_index_and_kind(tiles.primitives[i]);
        switch index_and_kind.kind {
            case PRIM_RECT {
                let new_color = sdf_rect(index_and_kind.index, canvas_pos, index_and_kind.textured);
                color = mix(color, new_color, new_color.a);
            }
            case PRIM_GLYPH {
                let new_color = sdf_rect(index_and_kind.index, canvas_pos, index_and_kind.textured);
                color = mix(color, new_color, new_color.a);
            }
            case PRIM_LINE {
                let new_color = sdf_line(index_and_kind.index, canvas_pos);
                color = mix(color, new_color, new_color.a);
            }
            default {}
        }
    }

    return color;
}
