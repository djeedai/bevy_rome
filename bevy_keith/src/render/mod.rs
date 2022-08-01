use std::{ops::Range, primitive};

use bevy::{
    asset::{Asset, AssetEvent, Handle, HandleId},
    core::{Pod, Zeroable},
    core_pipeline::core_2d::Transparent2d,
    ecs::{
        component::Component,
        entity::Entity,
        system::{
            lifetimeless::{Read, SQuery, SRes},
            Commands, Query, Res, ResMut, SystemParamItem,
        },
        world::{FromWorld, World},
    },
    math::Mat2,
    prelude::*,
    reflect::Uuid,
    render::{MainWorld,
        render_asset::RenderAssets,
        render_phase::{
            BatchedPhaseItem, DrawFunctions, EntityRenderCommand, RenderCommand,
            RenderCommandResult, RenderPhase, SetItemPipeline, TrackedRenderPass,
        },
        render_resource::{
            BindGroup, BindGroupDescriptor, BindGroupEntry, BindGroupLayout,
            BindGroupLayoutDescriptor, BindGroupLayoutEntry, BindingResource, BindingType,
            BlendState, Buffer, BufferBinding, BufferBindingType, BufferInitDescriptor, BufferSize,
            BufferUsages, BufferVec, ColorTargetState, ColorWrites, FragmentState, FrontFace,
            IndexFormat, MultisampleState, PipelineCache, PolygonMode, PrimitiveState,
            PrimitiveTopology, RenderPipelineDescriptor, SamplerBindingType, ShaderStages,
            ShaderType, SpecializedRenderPipeline, SpecializedRenderPipelines, TextureFormat,
            TextureSampleType, TextureViewDimension, VertexBufferLayout, VertexFormat, VertexState,
            VertexStepMode,
        },
        renderer::{RenderDevice, RenderQueue},
        texture::{BevyDefault, Image},
        view::{Msaa, ViewUniform, ViewUniformOffset, ViewUniforms},
        Extract,
    },
    sprite::Rect as SRect,
    utils::{FloatOrd, HashMap},
    window::WindowId,
};
use copyless::VecHelper;

use crate::{
    canvas::{Canvas, PrimImpl, Primitive},
    text::{CanvasTextId, KeithTextPipeline},
    PRIMITIVE_SHADER_HANDLE,
};

const NIL_HANDLE_ID: HandleId = HandleId::new(Uuid::nil(), 0);

trait HandleIdExt {
    fn is_valid(&self) -> bool;
}

impl HandleIdExt for HandleId {
    fn is_valid(&self) -> bool {
        *self != NIL_HANDLE_ID
    }
}

pub type DrawPrimitive = (
    SetItemPipeline,
    SetPrimitiveViewBindGroup<0>,
    SetPrimitiveBufferBindGroup<1>,
    SetPrimitiveTextureBindGroup<2>,
    DrawPrimitiveBatch,
);

pub struct SetPrimitiveViewBindGroup<const I: usize>;

impl<const I: usize> EntityRenderCommand for SetPrimitiveViewBindGroup<I> {
    type Param = (SRes<PrimitiveMeta>, SQuery<Read<ViewUniformOffset>>);

    fn render<'w>(
        view: Entity,
        _item: Entity,
        (primitive_meta, view_query): SystemParamItem<'w, '_, Self::Param>,
        pass: &mut TrackedRenderPass<'w>,
    ) -> RenderCommandResult {
        trace!("SetPrimitiveViewBindGroup: I={}", I);
        let view_uniform = view_query.get(view).unwrap();
        pass.set_bind_group(
            I,
            primitive_meta
                .into_inner()
                .view_bind_group
                .as_ref()
                .unwrap(),
            &[view_uniform.offset],
        );
        RenderCommandResult::Success
    }
}

pub struct SetPrimitiveBufferBindGroup<const I: usize>;

impl<const I: usize> EntityRenderCommand for SetPrimitiveBufferBindGroup<I> {
    type Param = (SRes<PrimitiveMeta>, SQuery<Read<PrimitiveBatch>>);

    fn render<'w>(
        _view: Entity,
        item: Entity,
        (primitive_meta, batch_query): SystemParamItem<'w, '_, Self::Param>,
        pass: &mut TrackedRenderPass<'w>,
    ) -> RenderCommandResult {
        let primitive_batch = batch_query.get(item).unwrap();
        trace!(
            "SetPrimitiveBufferBindGroup: I={} item={:?} canvas_entity={:?}",
            I,
            item,
            primitive_batch.canvas_entity
        );
        if let Some(canvas_meta) = primitive_meta
            .into_inner()
            .canvas_meta
            .get(&primitive_batch.canvas_entity)
        {
            pass.set_bind_group(I, &canvas_meta.primitive_bind_group, &[]);
            trace!("SetPrimitiveBufferBindGroup: SUCCESS");
            RenderCommandResult::Success
        } else {
            error!("SetPrimitiveBufferBindGroup: FAILURE");
            RenderCommandResult::Failure
        }
    }
}

pub struct SetPrimitiveTextureBindGroup<const I: usize>;

impl<const I: usize> EntityRenderCommand for SetPrimitiveTextureBindGroup<I> {
    type Param = (SRes<ImageBindGroups>, SQuery<Read<PrimitiveBatch>>);

    fn render<'w>(
        _view: Entity,
        item: Entity,
        (image_bind_groups, query_batch): SystemParamItem<'w, '_, Self::Param>,
        pass: &mut TrackedRenderPass<'w>,
    ) -> RenderCommandResult {
        let primitive_batch = query_batch.get(item).unwrap();
        trace!(
            "SetPrimitiveTextureBindGroup: I={} image={:?}",
            I,
            primitive_batch.image_handle_id
        );
        if primitive_batch.image_handle_id.is_valid() {
            let image_bind_groups = image_bind_groups.into_inner();
            trace!("image_bind_groups:");
            for (handle, bind_group) in &image_bind_groups.values {
                trace!("+ ibg: {:?} = {:?}", handle, bind_group);
            }
            pass.set_bind_group(
                I,
                image_bind_groups
                    .values
                    .get(&Handle::weak(primitive_batch.image_handle_id))
                    .unwrap(),
                &[],
            );
        }
        RenderCommandResult::Success
    }
}

pub struct DrawPrimitiveBatch;

impl<P: BatchedPhaseItem> RenderCommand<P> for DrawPrimitiveBatch {
    type Param = (SRes<PrimitiveMeta>, SQuery<Read<PrimitiveBatch>>);

    fn render<'w>(
        _view: Entity,
        item: &P,
        (primitive_meta, query_batch): SystemParamItem<'w, '_, Self::Param>,
        pass: &mut TrackedRenderPass<'w>,
    ) -> RenderCommandResult {
        let primitive_batch = query_batch.get(item.entity()).unwrap();
        let primitive_meta = primitive_meta.into_inner();
        if let Some(canvas_meta) = primitive_meta
            .canvas_meta
            .get(&primitive_batch.canvas_entity)
        {
            pass.set_index_buffer(
                canvas_meta.index_buffer.slice(..),
                0u64,
                IndexFormat::Uint32,
            );
            let indices = item.batch_range().as_ref().unwrap().clone();
            trace!("DrawPrimitiveBatch: indices={:?}", indices);
            pass.draw_indexed(indices, 0, 0..1);
            RenderCommandResult::Success
        } else {
            error!(
                "DrawPrimitiveBatch: Cannot find canvas meta for canvas entity {:?}",
                primitive_batch.canvas_entity
            );
            RenderCommandResult::Failure
        }
    }
}

#[derive(Component, Clone)]
pub struct PrimitiveBatch {
    /// Handle of the texture for the batch, or [`NIL_HANDLE_ID`] if not textured.
    image_handle_id: HandleId,
    /// Entity holding the [`Canvas`] component this batch is built from.
    canvas_entity: Entity,
    /// Index range.
    range: Range<u32>,
}

impl PrimitiveBatch {
    pub fn try_merge(&mut self, other: &PrimitiveBatch) -> bool {
        if self.image_handle_id == other.image_handle_id
            && self.canvas_entity == other.canvas_entity
            && self.range.end == other.range.start
        {
            self.range = self.range.start..other.range.end;
            true
        } else {
            false
        }
    }
}

pub struct CanvasMeta {
    /// Entity the `Canvas` component is attached to.
    canvas_entity: Entity,
    /// Bind group for the primitive buffer used by the canvas.
    primitive_bind_group: BindGroup,
    /// Index buffer for drawing all the primitives of the entire canvas.
    index_buffer: Buffer,
}
pub struct PrimitiveMeta {
    view_bind_group: Option<BindGroup>,
    /// Map from canvas entity to per-canvas meta.
    canvas_meta: HashMap<Entity, CanvasMeta>,
}

impl Default for PrimitiveMeta {
    fn default() -> Self {
        Self {
            view_bind_group: None,
            canvas_meta: HashMap::new(),
        }
    }
}

#[derive(Default)]
pub struct ImageBindGroups {
    values: HashMap<Handle<Image>, BindGroup>,
}

pub struct PrimitivePipeline {
    view_layout: BindGroupLayout,
    prim_layout: BindGroupLayout,
    material_layout: BindGroupLayout,
}

impl FromWorld for PrimitivePipeline {
    fn from_world(world: &mut World) -> Self {
        let world = world.cell();
        let render_device = world.get_resource::<RenderDevice>().unwrap();

        let view_layout = render_device.create_bind_group_layout(&BindGroupLayoutDescriptor {
            entries: &[BindGroupLayoutEntry {
                binding: 0,
                visibility: ShaderStages::VERTEX | ShaderStages::FRAGMENT,
                ty: BindingType::Buffer {
                    ty: BufferBindingType::Uniform,
                    has_dynamic_offset: true,
                    min_binding_size: Some(ViewUniform::min_size()),
                },
                count: None,
            }],
            label: Some("canvas_view_layout"),
        });

        let prim_layout = render_device.create_bind_group_layout(&BindGroupLayoutDescriptor {
            entries: &[BindGroupLayoutEntry {
                binding: 0,
                visibility: ShaderStages::VERTEX | ShaderStages::FRAGMENT,
                ty: BindingType::Buffer {
                    ty: BufferBindingType::Storage { read_only: true },
                    has_dynamic_offset: false,
                    min_binding_size: BufferSize::new(4_u64), // f32
                },
                count: None,
            }],
            label: Some("canvas_prim_layout"),
        });

        let material_layout = render_device.create_bind_group_layout(&BindGroupLayoutDescriptor {
            entries: &[
                BindGroupLayoutEntry {
                    binding: 0,
                    visibility: ShaderStages::FRAGMENT,
                    ty: BindingType::Texture {
                        multisampled: false,
                        sample_type: TextureSampleType::Float { filterable: true },
                        view_dimension: TextureViewDimension::D2,
                    },
                    count: None,
                },
                BindGroupLayoutEntry {
                    binding: 1,
                    visibility: ShaderStages::FRAGMENT,
                    ty: BindingType::Sampler(SamplerBindingType::Filtering),
                    count: None,
                },
            ],
            label: Some("quad_material_layout"),
        });

        PrimitivePipeline {
            view_layout,
            prim_layout,
            material_layout,
        }
    }
}

bitflags::bitflags! {
    #[repr(transparent)]
    // NOTE: Apparently quadro drivers support up to 64x MSAA.
    // MSAA uses the highest 6 bits for the MSAA sample count - 1 to support up to 64x MSAA.
    pub struct PrimitivePipelineKey: u32 {
        const NONE                        = 0;
        const TEXTURED                    = (1 << 0);
        const MSAA_RESERVED_BITS          = PrimitivePipelineKey::MSAA_MASK_BITS << PrimitivePipelineKey::MSAA_SHIFT_BITS;
    }
}

impl PrimitivePipelineKey {
    const MSAA_MASK_BITS: u32 = 0b111111;
    const MSAA_SHIFT_BITS: u32 = 32 - 6;

    pub fn from_msaa_samples(msaa_samples: u32) -> Self {
        let msaa_bits = ((msaa_samples - 1) & Self::MSAA_MASK_BITS) << Self::MSAA_SHIFT_BITS;
        PrimitivePipelineKey::from_bits(msaa_bits).unwrap()
    }

    pub fn msaa_samples(&self) -> u32 {
        ((self.bits >> Self::MSAA_SHIFT_BITS) & Self::MSAA_MASK_BITS) + 1
    }
}

impl SpecializedRenderPipeline for PrimitivePipeline {
    type Key = PrimitivePipelineKey;

    fn specialize(&self, key: Self::Key) -> RenderPipelineDescriptor {
        let mut layouts = vec![self.view_layout.clone(), self.prim_layout.clone()];
        let mut shader_defs = Vec::new();

        if key.contains(PrimitivePipelineKey::TEXTURED) {
            shader_defs.push("TEXTURED".to_string());
            layouts.push(self.material_layout.clone());
        }

        RenderPipelineDescriptor {
            vertex: VertexState {
                shader: PRIMITIVE_SHADER_HANDLE.typed::<Shader>(),
                entry_point: "vertex".into(),
                shader_defs: shader_defs.clone(),
                buffers: vec![], // vertex-less rendering
            },
            fragment: Some(FragmentState {
                shader: PRIMITIVE_SHADER_HANDLE.typed::<Shader>(),
                shader_defs,
                entry_point: "fragment".into(),
                targets: vec![Some(ColorTargetState {
                    format: TextureFormat::bevy_default(),
                    blend: Some(BlendState::ALPHA_BLENDING),
                    write_mask: ColorWrites::ALL,
                })],
            }),
            layout: Some(layouts),
            primitive: PrimitiveState {
                front_face: FrontFace::Ccw,
                cull_mode: None,
                unclipped_depth: false,
                polygon_mode: PolygonMode::Fill,
                conservative: false,
                topology: PrimitiveTopology::TriangleList,
                strip_index_format: None,
            },
            depth_stencil: None,
            multisample: MultisampleState {
                count: key.msaa_samples(),
                mask: !0,
                alpha_to_coverage_enabled: false,
            },
            label: Some("primitive_pipeline".into()),
        }
    }
}

const QUAD_INDICES: [usize; 6] = [0, 2, 3, 0, 1, 2];

const QUAD_VERTEX_POSITIONS: [Vec2; 4] = [
    Vec2::from_array([-0.5, -0.5]),
    Vec2::from_array([0.5, -0.5]),
    Vec2::from_array([0.5, 0.5]),
    Vec2::from_array([-0.5, 0.5]),
];

const QUAD_UVS: [Vec2; 4] = [
    Vec2::from_array([0., 1.]),
    Vec2::from_array([1., 1.]),
    Vec2::from_array([1., 0.]),
    Vec2::from_array([0., 0.]),
];

#[derive(Default)]
pub struct ExtractedCanvas {
    pub transform: GlobalTransform,
    pub primitives: Vec<Primitive>,
    storage: Option<Buffer>,
    storage_capacity: usize,
    index_buffer: Option<Buffer>,
    index_buffer_capacity: usize,
    pub index_count: u32,
    /// Scale factor of the window where this canvas is rendered.
    pub scale_factor: f32,
    /// Extracted data for all texts in use, in local text ID order.
    pub(crate) texts: Vec<ExtractedText>,
}

impl ExtractedCanvas {
    /// Write the CPU scratch buffer into the associated GPU storage buffer.
    pub fn write_buffers(
        &mut self,
        primitives: &[f32],
        indices: &[u32],
        render_device: &RenderDevice,
        render_queue: &RenderQueue,
    ) {
        trace!(
            "Writing {} primitive elements and {} indices to GPU buffers",
            primitives.len(),
            indices.len()
        );

        // Primitive buffer
        let size = primitives.len(); // FIXME - cap size to reasonable value
        let contents = bytemuck::cast_slice(&primitives[..]);
        if size > self.storage_capacity {
            // GPU buffer too small; reallocated...
            trace!(
                "Reallocate canvas_primitive_buffer: {} -> {}",
                self.storage_capacity,
                size
            );
            self.storage = Some(
                render_device.create_buffer_with_data(&BufferInitDescriptor {
                    label: Some("canvas_primitive_buffer"),
                    usage: BufferUsages::COPY_DST | BufferUsages::STORAGE,
                    contents,
                }),
            );
            self.storage_capacity = size;
        } else if let Some(storage) = &self.storage {
            // Write directly to existing GPU buffer
            render_queue.write_buffer(storage, 0, contents);
        }

        // Index buffer
        let size = indices.len(); // FIXME - cap size to reasonable value
        self.index_count += size as u32;
        let contents = bytemuck::cast_slice(&indices[..]);
        if size > self.index_buffer_capacity {
            // GPU buffer too small; reallocated...
            self.index_buffer = Some(render_device.create_buffer_with_data(
                &BufferInitDescriptor {
                    label: Some("canvas_index_buffer"),
                    usage: BufferUsages::COPY_DST | BufferUsages::INDEX,
                    contents,
                },
            ));
            self.index_buffer_capacity = size;
        } else if let Some(index_buffer) = &self.index_buffer {
            // Write directly to existing GPU buffer
            render_queue.write_buffer(index_buffer, 0, contents);
        }
    }

    #[inline]
    pub fn binding(&self) -> Option<BindingResource> {
        self.storage.as_ref().map(|buffer| {
            BindingResource::Buffer(BufferBinding {
                buffer: &buffer,
                offset: 0,
                size: None,
            })
        })
    }
}

/// Resource attached to the render world and containing all the data extracted from the
/// various visible [`Canvas`] components.
#[derive(Default)]
pub struct ExtractedCanvases {
    /// Map from app world's entity with a [`Canvas`] component to associated render world's
    /// extracted canvas.
    pub canvases: HashMap<Entity, ExtractedCanvas>,
}

#[derive(Default)]
pub struct PrimitiveAssetEvents {
    pub images: Vec<AssetEvent<Image>>,
}

/// Clone an [`AssetEvent`] manually by unwrapping and re-wrapping it, returning an event
/// with a weak handle.
///
/// This is necessary because [`AssetEvent`] is `!Clone`.
#[inline]
fn clone_asset_event_weak<T: Asset>(event: &AssetEvent<T>) -> AssetEvent<T> {
    match event {
        AssetEvent::Created { handle } => AssetEvent::Created {
            handle: handle.clone_weak(),
        },
        AssetEvent::Modified { handle } => AssetEvent::Modified {
            handle: handle.clone_weak(),
        },
        AssetEvent::Removed { handle } => AssetEvent::Removed {
            handle: handle.clone_weak(),
        },
    }
}

/// Render app system consuming asset events for [`Image`] components to react
/// to changes to the content of primitive textures.
pub(crate) fn extract_primitive_events(
    mut events: ResMut<PrimitiveAssetEvents>,
    mut image_events: Extract<EventReader<AssetEvent<Image>>>,
) {
    //trace!("extract_primitive_events");

    let PrimitiveAssetEvents { ref mut images } = *events;

    images.clear();

    for image in image_events.iter() {
        images.push(clone_asset_event_weak(image));
    }
}

#[derive(Debug, Default)]
pub(crate) struct ExtractedText {
    pub glyphs: Vec<ExtractedGlyph>,
}

#[derive(Debug)]
pub(crate) struct ExtractedGlyph {
    /// Offset of the glyph from the text origin.
    pub offset: Vec2,
    pub size: Vec2,
    /// Glyph color, as RGBA linear (0xAABBGGRR in little endian).
    pub color: u32,
    /// Handle of the atlas texture where the glyph is stored.
    pub handle_id: HandleId,
    /// Rectangle in UV coordinates delimiting the glyph area in the atlas texture.
    pub uv_rect: bevy::sprite::Rect,
}

/// Render app system extracting all primitives from all [`Canvas`] components, for later
/// rendering.
///
/// # Dependent components
///
/// [`Canvas`] components require at least a [`GlobalTransform`] component attached to the
/// same entity and describing the canvas 3D transform.
///
/// An optional [`Visibility`] component can be added to that same entity to dynamically
/// control the canvas visibility. By default if absent the canvas is assumed visible.
pub(crate) fn extract_primitives(
    mut extracted_canvases: ResMut<ExtractedCanvases>,
    texture_atlases: Extract<Res<Assets<TextureAtlas>>>,
    windows: Extract<Res<Windows>>,
    text_pipeline: Extract<Res<KeithTextPipeline>>,
    canvas_query: Extract<Query<(Entity, Option<&Visibility>, &Canvas, &GlobalTransform)>>,
) {
    trace!("extract_primitives");

    // TODO - handle multi-window
    let scale_factor = windows.scale_factor(WindowId::primary()) as f32;
    let inv_scale_factor = 1. / scale_factor;

    let mut extracted_canvases = &mut extracted_canvases.canvases;

    extracted_canvases.clear();

    for (entity, maybe_visibility, canvas, transform) in canvas_query.iter() {
        // Skip hidden canvases
        if !maybe_visibility.map_or(true, |vis| vis.is_visible) {
            continue;
        }

        // Swap render and main app primitive buffer
        let primitives = canvas.buffer().clone();
        trace!(
            "Canvas on Entity {:?} has {} primitives and {} text layouts",
            entity,
            primitives.len(),
            canvas.text_layouts().len(),
        );
        if primitives.is_empty() {
            continue;
        }

        // Process text glyphs. This requires access to various assets on the main app,
        // so needs to be done during the extract phase.
        let mut extracted_texts: Vec<ExtractedText> = vec![];
        for text in canvas.text_layouts() {
            let text_id = CanvasTextId::from_raw(entity, text.id);
            trace!("Extracting text {:?}...", text_id);

            if let Some(text_layout) = text_pipeline.get_glyphs(&text_id) {
                let width = text_layout.size.x * inv_scale_factor;
                let height = text_layout.size.y * inv_scale_factor;

                trace!(
                    "-> {} glyphs, w={} h={} scale={}",
                    text_layout.glyphs.len(),
                    width,
                    height,
                    scale_factor
                );

                let alignment_offset = match text.alignment.vertical {
                    VerticalAlign::Top => Vec2::new(0.0, -height),
                    VerticalAlign::Center => Vec2::new(0.0, -height * 0.5),
                    VerticalAlign::Bottom => Vec2::ZERO,
                } + match text.alignment.horizontal {
                    HorizontalAlign::Left => Vec2::ZERO,
                    HorizontalAlign::Center => Vec2::new(-width * 0.5, 0.0),
                    HorizontalAlign::Right => Vec2::new(-width, 0.0),
                };

                // let mut text_transform = extracted_canvas.transform;
                // text_transform.scale *= inv_scale_factor;

                let mut extracted_glyphs = vec![];
                for text_glyph in &text_layout.glyphs {
                    trace!(
                        "glyph: position={:?} size={:?}",
                        text_glyph.position,
                        text_glyph.size
                    );
                    let color = text.sections[text_glyph.section_index]
                        .style
                        .color
                        .as_linear_rgba_u32();
                    let atlas = texture_atlases
                        .get(&text_glyph.atlas_info.texture_atlas)
                        .unwrap();
                    let handle = atlas.texture.clone_weak();
                    let index = text_glyph.atlas_info.glyph_index as usize;
                    let uv_rect = atlas.textures[index];

                    let glyph_offset = alignment_offset * scale_factor + text_glyph.position;

                    extracted_glyphs.push(ExtractedGlyph {
                        offset: glyph_offset,
                        size: text_glyph.size,
                        color,
                        handle_id: handle.id,
                        uv_rect,
                    });

                    // let glyph_transform = Transform::from_translation(
                    //     alignment_offset * scale_factor + text_glyph.position.extend(0.),
                    // );

                    // let transform = text_transform.mul_transform(glyph_transform);

                    // extracted_sprites.sprites.push(ExtractedSprite {
                    //     transform,
                    //     color,
                    //     rect,
                    //     custom_size: None,
                    //     image_handle_id: handle.id,
                    //     flip_x: false,
                    //     flip_y: false,
                    //     anchor: Anchor::Center.as_vec(),
                    // });
                }

                let index = text.id as usize;
                trace!(
                    "Inserting index={} with {} glyphs into extracted texts of len={}...",
                    index,
                    extracted_glyphs.len(),
                    extracted_texts.len(),
                );
                if index >= extracted_texts.len() {
                    extracted_texts.resize_with(index + 1, Default::default);
                }
                extracted_texts[index].glyphs = extracted_glyphs;
            } else {
                trace!("Glyphs not ready yet...");
            }
        }

        // Save extracted canvas
        let extracted_canvas = extracted_canvases.entry(entity).or_insert(ExtractedCanvas {
            transform: *transform,
            ..Default::default()
        });
        extracted_canvas.primitives = primitives;
        extracted_canvas.scale_factor = scale_factor;
        extracted_canvas.texts = extracted_texts;
    }
}

pub(crate) struct PrimIter<'a> {
    prim: Option<Primitive>,
    index: usize,
    texts: &'a [ExtractedText],
}

impl<'a> PrimIter<'a> {
    pub fn new(prim: Primitive, texts: &'a [ExtractedText]) -> Self {
        Self {
            prim: Some(prim),
            index: 0,
            texts,
        }
    }
}

impl<'a> Iterator for PrimIter<'a> {
    type Item = (HandleId, u32);

    fn next(&mut self) -> Option<Self::Item> {
        if let Some(prim) = self.prim {
            let (_, idx_size) = prim.sizes(self.texts);
            let idx_size = idx_size as u32;
            match prim {
                Primitive::Text(text) => {
                    if text.id as usize >= self.texts.len() {
                        return None; // not ready
                    }
                    let text = &self.texts[text.id as usize];
                    if self.index < self.texts.len() {
                        let image_handle_id = text.glyphs[self.index].handle_id;
                        self.index += 1;
                        Some((image_handle_id, idx_size))
                    } else {
                        self.prim = None;
                        None
                    }
                }
                _ => {
                    self.prim = None;
                    // Currently all other primitives are non-textured
                    Some((NIL_HANDLE_ID, idx_size))
                }
            }
        } else {
            None
        }
    }
}

pub(crate) fn prepare_primitives(
    mut commands: Commands,
    mut extracted_canvases: ResMut<ExtractedCanvases>,
    render_device: Res<RenderDevice>,
    render_queue: Res<RenderQueue>,
    gpu_images: Res<RenderAssets<Image>>,
    mut image_bind_groups: ResMut<ImageBindGroups>,
    primitive_pipeline: Res<PrimitivePipeline>,
    events: Res<PrimitiveAssetEvents>,
) {
    trace!("prepare_primitives");

    // If an Image has changed, the GpuImage has (probably) changed
    for event in &events.images {
        match event {
            AssetEvent::Created { .. } => None,
            AssetEvent::Modified { handle } | AssetEvent::Removed { handle } => {
                trace!("Remove IBG for handle {:?} due to {:?}", handle, event);
                image_bind_groups.values.remove(handle)
            }
        };
    }

    let mut extracted_canvases = &mut extracted_canvases.canvases;

    for (entity, extracted_canvas) in extracted_canvases {
        trace!(
            "Canvas on Entity {:?} has {} primitives and {} texts",
            entity,
            extracted_canvas.primitives.len(),
            extracted_canvas.texts.len(),
        );

        extracted_canvas.index_count = 0;
        let mut primitives = vec![];
        let mut indices = vec![];

        // Impossible batching starting values that will be replaced on the first iteration
        let mut current_batch = PrimitiveBatch {
            image_handle_id: HandleId::Id(Uuid::nil(), u64::MAX),
            canvas_entity: *entity,
            range: 0..0,
        };

        // Serialize primitives into a binary float32 array, to work around the fact wgpu doesn't
        // have byte arrays. And f32 being the most common type of data in primitives limits the
        // amount of bitcast in the shader.
        for prim in &extracted_canvas.primitives {
            let offset = primitives.len() as u32;

            // Serialize the primitive
            let (prim_size, idx_size) = prim.sizes(&extracted_canvas.texts[..]);
            trace!("=> ps={} is={}", prim_size, idx_size);
            if prim_size > 0 && idx_size > 0 {
                primitives.reserve(prim_size);
                indices.reserve(idx_size);
                let prim_slice = primitives.spare_capacity_mut();
                let idx_slice = indices.spare_capacity_mut();

                // Write primitives and indices
                prim.write(
                    &extracted_canvas.texts[..],
                    &mut prim_slice[..prim_size],
                    offset,
                    &mut idx_slice[..idx_size],
                    extracted_canvas.scale_factor,
                );

                // Apply new storage sizes
                let new_prim_size = primitives.len() + prim_size;
                unsafe { primitives.set_len(new_prim_size) };
                let new_idx_size = indices.len() + idx_size;
                unsafe { indices.set_len(new_idx_size) };

                trace!("New primitive elements:");
                for f in &primitives[new_prim_size - prim_size..new_prim_size] {
                    trace!("+ f32[] = {}", f);
                }
                trace!("New indices:");
                for u in &indices[new_idx_size - idx_size..new_idx_size] {
                    trace!("+ u32[] = {:x}", u);
                }
            }

            // Loop on sub-primitives; Text primitives expand to one Rect primitive
            // per glyph, each of which _can_ have a separate atlas texture so potentially
            // can split the draw into a new batch.
            let batch_iter = PrimIter::new(*prim, &extracted_canvas.texts);
            for (image_handle_id, num_indices) in batch_iter {
                let new_batch = PrimitiveBatch {
                    image_handle_id,
                    canvas_entity: *entity,
                    range: current_batch.range.end..current_batch.range.end + num_indices,
                };
                trace!(
                    "New Batch: canvas_entity={:?} index={:?} handle={:?}",
                    new_batch.canvas_entity,
                    new_batch.range,
                    new_batch.image_handle_id
                );

                if current_batch.try_merge(&new_batch) {
                    assert_eq!(current_batch.range.end, new_batch.range.end);
                    trace!(
                        "Merged new batch with current batch: index={:?}",
                        current_batch.range
                    );
                    continue;
                }

                // Batches are different; output the previous one before starting a new one.

                // Check if the previous batch image is available on GPU; if so output the batch
                if !current_batch.image_handle_id.is_valid() {
                    trace!(
                        "Spawning batch: canvas_entity={:?} index={:?} (no tex)",
                        current_batch.canvas_entity,
                        current_batch.range,
                    );
                    commands.spawn_bundle((current_batch,));
                } else if let Some(gpu_image) =
                    gpu_images.get(&Handle::weak(current_batch.image_handle_id))
                {
                    image_bind_groups
                        .values
                        .entry(Handle::weak(current_batch.image_handle_id))
                        .or_insert_with(|| {
                            trace!(
                                "Insert new bind group for handle={:?}",
                                current_batch.image_handle_id
                            );
                            render_device.create_bind_group(&BindGroupDescriptor {
                                entries: &[
                                    BindGroupEntry {
                                        binding: 0,
                                        resource: BindingResource::TextureView(
                                            &gpu_image.texture_view,
                                        ),
                                    },
                                    BindGroupEntry {
                                        binding: 1,
                                        resource: BindingResource::Sampler(&gpu_image.sampler),
                                    },
                                ],
                                label: Some("primitive_material_bind_group"),
                                layout: &primitive_pipeline.material_layout,
                            })
                        });

                    trace!(
                        "Spawning batch: canvas_entity={:?} index={:?} handle={:?}",
                        current_batch.canvas_entity,
                        current_batch.range,
                        current_batch.image_handle_id,
                    );
                    commands.spawn_bundle((current_batch,));
                } else if !current_batch.range.is_empty() {
                    trace!(
                        "Ignoring current batch index={:?}: GPU texture not ready.",
                        current_batch.range
                    );
                }

                current_batch = new_batch;
            }
        }

        // Output the last batch
        // FIXME - merge duplicated code with above
        if !current_batch.range.is_empty() {
            // Check if the previous batch image is available on GPU; if so output the batch
            if !current_batch.image_handle_id.is_valid() {
                trace!(
                    "Spawning batch: canvas_entity={:?} index={:?} (no tex)",
                    current_batch.canvas_entity,
                    current_batch.range,
                );
                commands.spawn_bundle((current_batch,));
            } else if let Some(gpu_image) =
                gpu_images.get(&Handle::weak(current_batch.image_handle_id))
            {
                image_bind_groups
                    .values
                    .entry(Handle::weak(current_batch.image_handle_id))
                    .or_insert_with(|| {
                        trace!(
                            "Insert new bind group for handle={:?}",
                            current_batch.image_handle_id
                        );
                        render_device.create_bind_group(&BindGroupDescriptor {
                            entries: &[
                                BindGroupEntry {
                                    binding: 0,
                                    resource: BindingResource::TextureView(&gpu_image.texture_view),
                                },
                                BindGroupEntry {
                                    binding: 1,
                                    resource: BindingResource::Sampler(&gpu_image.sampler),
                                },
                            ],
                            label: Some("primitive_material_bind_group"),
                            layout: &primitive_pipeline.material_layout,
                        })
                    });

                trace!(
                    "Spawning batch: canvas_entity={:?} index={:?} handle={:?}",
                    current_batch.canvas_entity,
                    current_batch.range,
                    current_batch.image_handle_id,
                );
                commands.spawn_bundle((current_batch,));
            } else if !current_batch.range.is_empty() {
                trace!(
                    "Ignoring current batch index={:?}: GPU texture not ready.",
                    current_batch.range
                );
            }
        }

        // Upload to GPU buffers
        if primitives.len() > 0 && indices.len() > 0 {
            trace!(
                "Writing {} elems and {} indices for Canvas of entity {:?}",
                primitives.len(),
                indices.len(),
                entity
            );
            extracted_canvas.write_buffers(
                &primitives[..],
                &indices[..],
                &render_device,
                &render_queue,
            );
        }
    }
}

#[allow(clippy::too_many_arguments)]
pub fn queue_primitives(
    draw_functions: Res<DrawFunctions<Transparent2d>>,
    render_device: Res<RenderDevice>,
    _render_queue: Res<RenderQueue>,
    mut primitive_meta: ResMut<PrimitiveMeta>,
    view_uniforms: Res<ViewUniforms>,
    primitive_pipeline: Res<PrimitivePipeline>,
    mut pipelines: ResMut<SpecializedRenderPipelines<PrimitivePipeline>>,
    mut pipeline_cache: ResMut<PipelineCache>,
    _gpu_images: Res<RenderAssets<Image>>,
    msaa: Res<Msaa>,
    mut extracted_canvases: ResMut<ExtractedCanvases>,
    mut views: Query<&mut RenderPhase<Transparent2d>>,
    batches: Query<(Entity, &PrimitiveBatch)>,
) {
    trace!("queue_primitives");

    let view_binding = match view_uniforms.uniforms.binding() {
        Some(view_binding) => view_binding,
        None => {
            return;
        }
    };

    let primitive_meta = &mut primitive_meta;

    primitive_meta.view_bind_group = Some(render_device.create_bind_group(&BindGroupDescriptor {
        entries: &[BindGroupEntry {
            binding: 0,
            resource: view_binding,
        }],
        label: Some("primitive_view_bind_group"),
        layout: &primitive_pipeline.view_layout,
    }));

    // TODO - per view culling?! (via VisibleEntities)
    let draw_primitives_function = draw_functions.read().get_id::<DrawPrimitive>().unwrap();
    let key = PrimitivePipelineKey::from_msaa_samples(msaa.samples);
    let untextured_pipeline = pipelines.specialize(&mut pipeline_cache, &primitive_pipeline, key);
    let textured_pipeline = pipelines.specialize(
        &mut pipeline_cache,
        &primitive_pipeline,
        key | PrimitivePipelineKey::TEXTURED,
    );

    for (batch_entity, batch) in batches.iter() {
        if batch.range.is_empty() {
            continue;
        }

        let canvas_entity = batch.canvas_entity;

        let is_textured = batch.image_handle_id.is_valid();
        let pipeline = if is_textured {
            textured_pipeline
        } else {
            untextured_pipeline
        };

        let extracted_canvas =
            if let Some(extracted_canvas) = extracted_canvases.canvases.get(&canvas_entity) {
                extracted_canvas
            } else {
                continue;
            };

        let primitive_bind_group = render_device.create_bind_group(&BindGroupDescriptor {
            entries: &[BindGroupEntry {
                binding: 0,
                resource: extracted_canvas.binding().unwrap(),
            }],
            label: Some("prim_bind_group"),
            layout: &primitive_pipeline.prim_layout,
        });
        let index_buffer = extracted_canvas.index_buffer.as_ref().unwrap().clone();

        // Update meta map
        primitive_meta
            .canvas_meta
            .entry(canvas_entity)
            .and_modify(|canvas_meta| {
                //canvas_meta.batch_entity = batch_entity;
                canvas_meta.primitive_bind_group = primitive_bind_group.clone();
                canvas_meta.index_buffer = index_buffer.clone();
            })
            .or_insert_with(|| {
                trace!("Adding new CanvasMeta: canvas_entity={:?}", canvas_entity);
                CanvasMeta {
                    canvas_entity,
                    primitive_bind_group,
                    index_buffer,
                }
            });

        trace!(
            "CanvasMeta: canvas_entity={:?} batch_entity={:?} textured={}",
            canvas_entity,
            batch_entity,
            is_textured,
        );

        let sort_key = FloatOrd(extracted_canvas.transform.translation().z);

        // FIXME - Use VisibleEntities to optimize per-view
        for mut transparent_phase in views.iter_mut() {
            trace!(
                "Add Transparent2d item: {:?} (sort={:?})",
                batch.range,
                sort_key
            );
            transparent_phase.add(Transparent2d {
                draw_function: draw_primitives_function,
                pipeline,
                entity: batch_entity,
                sort_key,
                batch_range: Some(batch.range.clone()),
            });
        }
    }
}