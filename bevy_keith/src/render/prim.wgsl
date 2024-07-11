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
}

struct Extras {
    color: vec4<f32>,
    radius: f32,
}

struct Rect {
    center: vec2<f32>,
    half_size: vec2<f32>,
    extras: Extras,
}

struct Line {
    p0: vec2<f32>,
    p1: vec2<f32>,
    thickness: f32,
    extras: Extras,
}

struct QPie {
    origin: vec2<f32>,
    radii: vec2<f32>,
    extras: Extras,
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

struct PrimitiveInfo {
    index: u32,
    kind: u32,
    textured: bool,
    bordered: bool,
}

fn unpack_primitive_index(value: u32) -> PrimitiveInfo {
    let index = (value & 0x07FFFFFFu);
    let bordered = (value & 0x08000000u) != 0u;
    let kind = (value & 0x70000000u) >> 28u;
    let textured = (value & 0x80000000u) != 0u;
    return PrimitiveInfo(index, kind, textured, bordered);
}

fn get_vertex_pos(vertex_index: u32) -> vec2<f32> {
    switch vertex_index {
        case 0u { return vec2<f32>(-1., -1.); }
        case 1u { return vec2<f32>(3., -1.); }
        case 2u { return vec2<f32>(-1., 3.); }
        default { return vec2<f32>(1e38, 1e38); }
    }
}

fn read_rect(offset: u32) -> Rect {
    var rect: Rect;

    let x = primitives.elems[offset];
    let y = primitives.elems[offset + 1u];
    rect.center = vec2<f32>(x, y);
    
    let hw = primitives.elems[offset + 2u];
    let hh = primitives.elems[offset + 3u];
    rect.half_size = vec2<f32>(hw, hh);

    rect.extras.radius = primitives.elems[offset + 4u];
    
    let c = primitives.elems[offset + 5u];
    let uc: u32 = bitcast<u32>(c);
    rect.extras.color = unpack4x8unorm(uc);

    return rect;
}

fn read_line(offset: u32) -> Line {
    var line: Line;

    let p0x = primitives.elems[offset];
    let p0y = primitives.elems[offset + 1u];
    line.p0 = vec2<f32>(p0x, p0y);
    
    let p1x = primitives.elems[offset + 2u];
    let p1y = primitives.elems[offset + 3u];
    line.p1 = vec2<f32>(p1x, p1y);

    let c = primitives.elems[offset + 4u];
    let uc: u32 = bitcast<u32>(c);
    line.extras.color = unpack4x8unorm(uc);

    line.thickness = primitives.elems[offset + 5u];

    line.extras.radius = 0.0; // TODO

    return line;
}

fn read_qpie(offset: u32) -> QPie {
    var qpie: QPie;

    let x = primitives.elems[offset];
    let y = primitives.elems[offset + 1u];
    qpie.origin = vec2<f32>(x, y);

    let rx = primitives.elems[offset + 2u];
    let ry = primitives.elems[offset + 3u];
    qpie.radii = vec2<f32>(rx, ry);

    let c = primitives.elems[offset + 4u];
    let uc: u32 = bitcast<u32>(c);
    qpie.extras.color = unpack4x8unorm(uc);

    qpie.extras.radius = 0.0; // TODO

    return qpie;
}

/// Signed distance to an axis-aligned rectangle.
fn sd_rect(p: vec2<f32>, rect: Rect) -> f32 {
    let delta = abs(p - rect.center) - rect.half_size + rect.extras.radius;
    return length(max(delta, vec2<f32>(0))) + max(min(delta.x, 0.), min(delta.y, 0.)) - rect.extras.radius;
}

/// Signed distance to an infinitely thin line segment.
fn sd_segment(p0: vec2<f32>, p1: vec2<f32>, p: vec2<f32>) -> f32 {
    let p0p = p - p0;
    let p01 = p1 - p0;
    let h = saturate(dot(p0p, p01) / dot(p01, p01));
    return length(p0p - p01 * h);
}

/// Signed distance to a line (thick segment).
fn sd_line(p0: vec2<f32>, p1: vec2<f32>, thickness: f32, p: vec2<f32>) -> f32 {
    let dir = p1 - p0;
    let d = normalize(dir);
    let center = p0 + dir / 2.;
    let rot_delta = mat2x2<f32>(d.x, -d.y, d.y, d.x) * (p - center);
    let delta = abs(rot_delta) - vec2<f32>(length(dir), thickness) * 0.5;
    return length(max(delta, vec2<f32>(0))) + max(min(delta.x, 0.), min(delta.y, 0.));
}

/// Calculate the anti-aliased coverage of a pixel based on its SDF distance.
fn aa_coverage(dist: f32) -> f32 {
    // The mathematical border is exactly at 'dist'. But we want a smooth edge between the two pixels
    // directly before and after the border. So we need to map the pixels at d=-0.5 and d=+0.5 to the
    // coverage values 1. and 0., respectively. This gives the best result for axis-aligned edges,
    // and gives an acceptable 1-px wide smoothing for all other edges.
    return smoothstep(1., 0., dist + 0.5);
}

fn sdf_rect(offset: u32, canvas_pos: vec2<f32>) -> vec4<f32> {
    let rect = read_rect(offset);
    let dist = sd_rect(canvas_pos, rect);
    let alpha = rect.extras.color.a * aa_coverage(dist);
    return vec4<f32>(rect.extras.color.rgb, alpha);
}

fn sdf_glyph(offset: u32, canvas_pos: vec2<f32>) -> vec4<f32> {
    let rect = read_rect(offset);
    let dist = sd_rect(canvas_pos, rect);
    let alpha = rect.extras.color.a * aa_coverage(dist);

    let uv_x = primitives.elems[offset + 6u];
    let uv_y = primitives.elems[offset + 7u];
    let uv_sx = primitives.elems[offset + 8u];
    let uv_sy = primitives.elems[offset + 9u];
    let uv_origin = vec2<f32>(uv_x, uv_y);
    let uv_scale = vec2<f32>(uv_sx, uv_sy);
    let uv = (canvas_pos - rect.center) * uv_scale + uv_origin;
    let tex = textureSample(quad_texture, quad_sampler, uv);

    return vec4<f32>(rect.extras.color.rgb, alpha * tex.a * rect.extras.color.a);
}

fn sdf_line(offset: u32, canvas_pos: vec2<f32>) -> vec4<f32> {
    let line = read_line(offset);
    let dist = sd_line(line.p0, line.p1, line.thickness, canvas_pos) - line.extras.radius;
    let alpha = line.extras.color.a * aa_coverage(dist);
    return vec4<f32>(line.extras.color.rgb, alpha);
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

    let canvas_pos = in.position.xy;
    var color = vec4<f32>();

    // Loop over all primitives for that tile, and accumulate color
    let prim_offset = offsets_and_counts[tile_index].offset;
    let prim_count = offsets_and_counts[tile_index].count;
    for (var i = prim_offset; i < prim_offset + prim_count; i += 1u) {
        let prim_info = unpack_primitive_index(tiles.primitives[i]);
        var offset: u32;
        var dist: f32;
        var new_color: vec4<f32>;
        var uv_origin: vec2<f32>;
        var coverage: f32;
        switch prim_info.kind {
            case PRIM_RECT {
                let rect = read_rect(prim_info.index);
                uv_origin = rect.center;
                dist = sd_rect(canvas_pos, rect);
                coverage = aa_coverage(dist);
                let alpha = rect.extras.color.a * coverage;
                new_color = vec4<f32>(rect.extras.color.rgb, alpha);
                offset = 6u + prim_info.index;
            }
            case PRIM_GLYPH {
                let rect = read_rect(prim_info.index);
                uv_origin = rect.center;
                dist = sd_rect(canvas_pos, rect);
                coverage = aa_coverage(dist);
                let alpha = rect.extras.color.a * coverage;

                let uv_x = primitives.elems[prim_info.index + 6u];
                let uv_y = primitives.elems[prim_info.index + 7u];
                let uv_sx = primitives.elems[prim_info.index + 8u];
                let uv_sy = primitives.elems[prim_info.index + 9u];
                let uv_origin0 = vec2<f32>(uv_x, uv_y);
                let uv_scale = vec2<f32>(uv_sx, uv_sy);
                let uv = (canvas_pos - rect.center) * uv_scale + uv_origin0;
                let tex = textureSample(quad_texture, quad_sampler, uv);

                new_color = vec4<f32>(rect.extras.color.rgb, alpha * tex.a * rect.extras.color.a);
                offset = 10u + prim_info.index;
            }
            case PRIM_LINE {
                let line = read_line(prim_info.index);
                dist = sd_line(line.p0, line.p1, line.thickness, canvas_pos) - line.extras.radius;
                coverage = aa_coverage(dist);
                let alpha = line.extras.color.a * coverage;
                new_color = vec4<f32>(line.extras.color.rgb, alpha);
                uv_origin = (line.p0 + line.p1) / 2.;
                offset = 6u + prim_info.index;
            }
            default {}
        }

        let new_alpha = mix(color.a, 1.0, new_color.a);
        let rgb = mix(color.rgb, new_color.rgb, new_color.a);
        color = vec4<f32>(rgb, new_alpha);
            
        var off = offset;
        if (prim_info.textured) {
            let uv_x = primitives.elems[off + 0u];
            let uv_y = primitives.elems[off + 1u];
            let uv_sx = primitives.elems[off + 2u];
            let uv_sy = primitives.elems[off + 3u];
            let uv_offset = vec2<f32>(uv_x, uv_y);
            let uv_scale = vec2<f32>(uv_sx, uv_sy);
            let uv = fma(canvas_pos - uv_origin, uv_scale, uv_offset);
            let tex_color = textureSample(quad_texture, quad_sampler, uv).rgb;
            color = vec4<f32>(tex_color * color.rgb, color.a);
            off += 4u;
        }

        if (prim_info.bordered) {
            let border_width = primitives.elems[off + 0u];
            let bc = primitives.elems[off + 1u];
            let ubc: u32 = bitcast<u32>(bc);
            let border_color = unpack4x8unorm(ubc);
            let dist2 = dist + border_width;
            let alpha2 = aa_coverage(dist2);
            color = vec4<f32>(mix(color.rgb, border_color.rgb, (1. - alpha2) * coverage), color.a);
        }
    }

    return color;
}
