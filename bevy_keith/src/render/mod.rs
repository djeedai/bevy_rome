use std::{fmt::Write, ops::Range};

use bevy::{
    asset::{Asset, AssetEvent, AssetId},
    core_pipeline::core_2d::Transparent2d,
    ecs::{
        component::Component,
        entity::Entity,
        query::ROQueryItem,
        system::{
            lifetimeless::{Read, SRes},
            Commands, Query, Res, ResMut, SystemParamItem,
        },
        world::{FromWorld, World},
    },
    prelude::*,
    render::{
        render_asset::RenderAssets,
        render_phase::{
            DrawFunctions, PhaseItem, RenderCommand, RenderCommandResult, RenderPhase,
            SetItemPipeline, TrackedRenderPass,
        },
        render_resource::{
            BindGroup, BindGroupEntry, BindGroupLayout, BindGroupLayoutEntry, BindingResource,
            BindingType, BlendState, Buffer, BufferBinding, BufferBindingType,
            BufferInitDescriptor, BufferSize, BufferUsages, ColorTargetState, ColorWrites,
            FragmentState, FrontFace, IndexFormat, MultisampleState, PipelineCache, PolygonMode,
            PrimitiveState, PrimitiveTopology, RenderPipelineDescriptor, SamplerBindingType,
            ShaderStages, ShaderType, SpecializedRenderPipeline, SpecializedRenderPipelines,
            TextureFormat, TextureSampleType, TextureViewDimension, VertexState,
        },
        renderer::{RenderDevice, RenderQueue},
        texture::{BevyDefault, Image},
        view::{Msaa, ViewUniform, ViewUniformOffset, ViewUniforms},
        Extract,
    },
    utils::{tracing::enabled, FloatOrd, HashMap},
    window::PrimaryWindow,
};

use crate::{
    canvas::{Canvas, PrimImpl, Primitive, PrimitiveInfo, TextPrimitive},
    text::CanvasTextId,
    PRIMITIVE_SHADER_HANDLE,
};

pub type DrawPrimitive = (
    SetItemPipeline,
    SetPrimitiveViewBindGroup<0>,
    SetPrimitiveBufferBindGroup<1>,
    SetPrimitiveTextureBindGroup<2>,
    DrawPrimitiveBatch,
);

pub struct SetPrimitiveViewBindGroup<const I: usize>;

impl<P: PhaseItem, const I: usize> RenderCommand<P> for SetPrimitiveViewBindGroup<I> {
    type Param = SRes<PrimitiveMeta>;
    type ViewQuery = Read<ViewUniformOffset>;
    type ItemQuery = ();

    fn render<'w>(
        _item: &P,
        view_uniform_offset: ROQueryItem<'w, Self::ViewQuery>,
        _entity: Option<()>,
        primitive_meta: SystemParamItem<'w, '_, Self::Param>,
        pass: &mut TrackedRenderPass<'w>,
    ) -> RenderCommandResult {
        trace!("SetPrimitiveViewBindGroup: I={}", I);
        let view_bind_group = primitive_meta
            .into_inner()
            .view_bind_group
            .as_ref()
            .unwrap();
        pass.set_bind_group(I, view_bind_group, &[view_uniform_offset.offset]);
        RenderCommandResult::Success
    }
}

pub struct SetPrimitiveBufferBindGroup<const I: usize>;

impl<P: PhaseItem, const I: usize> RenderCommand<P> for SetPrimitiveBufferBindGroup<I> {
    type Param = SRes<PrimitiveMeta>;
    type ViewQuery = ();
    type ItemQuery = Read<PrimitiveBatch>;

    fn render<'w>(
        _item: &P,
        _view: ROQueryItem<'w, Self::ViewQuery>,
        primitive_batch: Option<ROQueryItem<'w, Self::ItemQuery>>,
        primitive_meta: SystemParamItem<'w, '_, Self::Param>,
        pass: &mut TrackedRenderPass<'w>,
    ) -> RenderCommandResult {
        let Some(primitive_batch) = primitive_batch else {
            return RenderCommandResult::Failure;
        };
        trace!(
            "SetPrimitiveBufferBindGroup: I={} canvas_entity={:?}",
            I,
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

impl<P: PhaseItem, const I: usize> RenderCommand<P> for SetPrimitiveTextureBindGroup<I> {
    type Param = SRes<ImageBindGroups>;
    type ViewQuery = ();
    type ItemQuery = Read<PrimitiveBatch>;

    fn render<'w>(
        _item: &P,
        _view: ROQueryItem<'w, Self::ViewQuery>,
        primitive_batch: Option<ROQueryItem<'w, Self::ItemQuery>>,
        image_bind_groups: SystemParamItem<'w, '_, Self::Param>,
        pass: &mut TrackedRenderPass<'w>,
    ) -> RenderCommandResult {
        let Some(primitive_batch) = primitive_batch else {
            return RenderCommandResult::Failure;
        };
        trace!(
            "SetPrimitiveTextureBindGroup: I={} image={:?} (valid={})",
            I,
            primitive_batch.image_handle_id,
            if primitive_batch.image_handle_id != AssetId::<Image>::invalid() {
                "true"
            } else {
                "false"
            }
        );
        if primitive_batch.image_handle_id != AssetId::<Image>::invalid() {
            let image_bind_groups = image_bind_groups.into_inner();
            trace!("image_bind_groups:");
            for (handle, bind_group) in &image_bind_groups.values {
                trace!("+ ibg: {:?} = {:?}", handle, bind_group);
            }
            pass.set_bind_group(
                I,
                image_bind_groups
                    .values
                    .get(&primitive_batch.image_handle_id)
                    .unwrap(),
                &[],
            );
        }
        RenderCommandResult::Success
    }
}

pub struct DrawPrimitiveBatch;

impl<P: PhaseItem> RenderCommand<P> for DrawPrimitiveBatch {
    type Param = SRes<PrimitiveMeta>;
    type ViewQuery = ();
    type ItemQuery = Read<PrimitiveBatch>;

    fn render<'w>(
        _item: &P,
        _view: ROQueryItem<'w, Self::ViewQuery>,
        primitive_batch: Option<ROQueryItem<'w, Self::ItemQuery>>,
        primitive_meta: SystemParamItem<'w, '_, Self::Param>,
        pass: &mut TrackedRenderPass<'w>,
    ) -> RenderCommandResult {
        let Some(primitive_batch) = primitive_batch else {
            return RenderCommandResult::Failure;
        };
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
            let indices = primitive_batch.range.clone();
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

/// Batch of primitives sharing the same [`Canvas`] and rendering
/// characteristics, and which can be rendered with a single draw call.
#[derive(Component, Clone)]
pub struct PrimitiveBatch {
    /// Handle of the texture for the batch, or [`NIL_HANDLE_ID`] if not
    /// textured.
    image_handle_id: AssetId<Image>,
    /// Entity holding the [`Canvas`] component this batch is built from.
    canvas_entity: Entity,
    /// Index range.
    range: Range<u32>,
}

impl PrimitiveBatch {
    /// Create a batch with invalid values, that will never merge with anyhing.
    ///
    /// This is typically used as an initializing placeholder when doing
    /// incremental batching.
    pub fn invalid() -> Self {
        PrimitiveBatch {
            image_handle_id: AssetId::<Image>::invalid(),
            canvas_entity: Entity::PLACEHOLDER,
            range: 0..0,
        }
    }

    /// Try to merge a batch into the current batch.
    ///
    /// Return `true` if the batch was merged, or `false` otherwise.
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

/// Metadata for [`Canvas`] rendering.
struct CanvasMeta {
    /// Entity the [`Canvas`] component is attached to.
    canvas_entity: Entity,
    /// Bind group for the primitive buffer used by the canvas.
    primitive_bind_group: BindGroup,
    /// Index buffer for drawing all the primitives of the entire canvas.
    index_buffer: Buffer,
}

#[derive(Resource)]
pub struct PrimitiveMeta {
    view_bind_group: Option<BindGroup>,
    /// Map from an [`Entity`] with a [`Canvas`] component to the meta for that
    /// canvas.
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

/// Shader bind groups for all images currently in use by primitives.
#[derive(Default, Resource)]
pub struct ImageBindGroups {
    values: HashMap<AssetId<Image>, BindGroup>,
}

/// Rendering pipeline for [`Canvas`] primitives.
#[derive(Resource)]
pub struct PrimitivePipeline {
    /// Bind group layout for the uniform buffer containing the [`ViewUniform`]
    /// with the camera details of the current view being rendered.
    view_layout: BindGroupLayout,
    /// Bind group layout for the primitive buffer.
    prim_layout: BindGroupLayout,
    /// Bind group layout for the texture used by textured primitives.
    material_layout: BindGroupLayout,
}

impl FromWorld for PrimitivePipeline {
    fn from_world(world: &mut World) -> Self {
        let world = world.cell();
        let render_device = world.get_resource::<RenderDevice>().unwrap();

        let view_layout = render_device.create_bind_group_layout(
            "canvas_view_layout",
            &[BindGroupLayoutEntry {
                binding: 0,
                visibility: ShaderStages::VERTEX | ShaderStages::FRAGMENT,
                ty: BindingType::Buffer {
                    ty: BufferBindingType::Uniform,
                    has_dynamic_offset: true,
                    min_binding_size: Some(ViewUniform::min_size()),
                },
                count: None,
            }],
        );

        let prim_layout = render_device.create_bind_group_layout(
            "canvas_prim_layout",
            &[BindGroupLayoutEntry {
                binding: 0,
                visibility: ShaderStages::VERTEX | ShaderStages::FRAGMENT,
                ty: BindingType::Buffer {
                    ty: BufferBindingType::Storage { read_only: true },
                    has_dynamic_offset: false,
                    min_binding_size: BufferSize::new(4_u64), // f32
                },
                count: None,
            }],
        );

        let material_layout = render_device.create_bind_group_layout(
            "quad_material_layout",
            &[
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
        );

        PrimitivePipeline {
            view_layout,
            prim_layout,
            material_layout,
        }
    }
}

bitflags::bitflags! {
    #[repr(transparent)]
    #[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
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
        PrimitivePipelineKey::from_bits_retain(msaa_bits)
    }

    pub fn msaa_samples(&self) -> u32 {
        ((self.bits() >> Self::MSAA_SHIFT_BITS) & Self::MSAA_MASK_BITS) + 1
    }
}

impl SpecializedRenderPipeline for PrimitivePipeline {
    type Key = PrimitivePipelineKey;

    fn specialize(&self, key: Self::Key) -> RenderPipelineDescriptor {
        let mut layout = vec![self.view_layout.clone(), self.prim_layout.clone()];
        let mut shader_defs = Vec::new();

        if key.contains(PrimitivePipelineKey::TEXTURED) {
            shader_defs.push("TEXTURED".into());
            layout.push(self.material_layout.clone());
        }

        RenderPipelineDescriptor {
            vertex: VertexState {
                shader: PRIMITIVE_SHADER_HANDLE,
                entry_point: "vertex".into(),
                shader_defs: shader_defs.clone(),
                buffers: vec![], // vertex-less rendering
            },
            fragment: Some(FragmentState {
                shader: PRIMITIVE_SHADER_HANDLE,
                shader_defs,
                entry_point: "fragment".into(),
                targets: vec![Some(ColorTargetState {
                    format: TextureFormat::bevy_default(),
                    blend: Some(BlendState::ALPHA_BLENDING),
                    write_mask: ColorWrites::ALL,
                })],
            }),
            layout,
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
            push_constant_ranges: vec![],
        }
    }
}

/// Rendering data extracted from a single [`Canvas`] component.
#[derive(Default)]
pub struct ExtractedCanvas {
    /// Global transform of the canvas.
    pub transform: GlobalTransform,
    /// Collection of primitives rendered in this canvas.
    pub primitives: Vec<Primitive>,
    storage: Option<Buffer>,
    storage_capacity: usize,
    index_buffer: Option<Buffer>,
    index_buffer_capacity: usize,
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

/// Resource attached to the render world and containing all the data extracted
/// from the various visible [`Canvas`] components.
#[derive(Default, Resource)]
pub struct ExtractedCanvases {
    /// Map from app world's entity with a [`Canvas`] component to associated
    /// render world's extracted canvas.
    pub canvases: HashMap<Entity, ExtractedCanvas>,
}

#[derive(Default, Resource)]
pub struct PrimitiveAssetEvents {
    pub images: Vec<AssetEvent<Image>>,
}

/// Clone an [`AssetEvent`] manually by unwrapping and re-wrapping it, returning
/// an event with a weak handle.
///
/// This is necessary because [`AssetEvent`] is `!Clone`.
#[inline]
fn clone_asset_event_weak<T: Asset>(event: &AssetEvent<T>) -> AssetEvent<T> {
    match event {
        AssetEvent::Added { id } => AssetEvent::Added { id: *id },
        AssetEvent::Modified { id } => AssetEvent::Modified { id: *id },
        AssetEvent::Removed { id } => AssetEvent::Removed { id: *id },
        AssetEvent::LoadedWithDependencies { id } => AssetEvent::LoadedWithDependencies { id: *id },
        AssetEvent::Unused { id } => AssetEvent::Unused { id: *id },
    }
}

/// Render app system consuming asset events for [`Image`] components to react
/// to changes to the content of primitive textures.
pub(crate) fn extract_primitive_events(
    mut events: ResMut<PrimitiveAssetEvents>,
    mut image_events: Extract<EventReader<AssetEvent<Image>>>,
) {
    // trace!("extract_primitive_events");

    let PrimitiveAssetEvents { ref mut images } = *events;

    images.clear();

    for image in image_events.read() {
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
    /// Glyph color, as RGBA linear (0xAABBGGRR in little endian). Extracted
    /// from the text section's style ([`TextStyle::color`]).
    pub color: u32,
    /// Handle of the atlas texture where the glyph is stored.
    pub handle_id: AssetId<Image>,
    /// Rectangle in UV coordinates delimiting the glyph area in the atlas
    /// texture.
    pub uv_rect: bevy::math::Rect,
}

/// Render app system extracting all primitives from all [`Canvas`] components,
/// for later rendering.
///
/// # Dependent components
///
/// [`Canvas`] components require at least a [`GlobalTransform`] component
/// attached to the same entity and describing the canvas 3D transform.
///
/// An optional [`ComputedVisibility`] component can be added to that same
/// entity to dynamically control the canvas visibility. By default if absent
/// the canvas is assumed visible.
pub(crate) fn extract_primitives(
    mut extracted_canvases: ResMut<ExtractedCanvases>,
    texture_atlases: Extract<Res<Assets<TextureAtlasLayout>>>,
    q_window: Extract<Query<&Window, With<PrimaryWindow>>>,
    canvas_query: Extract<Query<(Entity, Option<&ViewVisibility>, &Canvas, &GlobalTransform)>>,
) {
    trace!("extract_primitives");

    // TODO - handle multi-window
    let Ok(primary_window) = q_window.get_single() else {
        return;
    };
    let scale_factor = primary_window.scale_factor() as f32;
    let inv_scale_factor = 1. / scale_factor;

    let extracted_canvases = &mut extracted_canvases.canvases;

    extracted_canvases.clear();

    for (entity, maybe_computed_visibility, canvas, transform) in canvas_query.iter() {
        // Skip hidden canvases. If no ComputedVisibility component is present, assume
        // visible.
        if !maybe_computed_visibility.map_or(true, |cvis| cvis.get()) {
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

            if let Some(text_layout_info) = &text.layout_info {
                let width = text_layout_info.logical_size.x * inv_scale_factor;
                let height = text_layout_info.logical_size.y * inv_scale_factor;

                let text_anchor = -(text.anchor.as_vec() + 0.5);
                let alignment_translation = text_layout_info.logical_size * text_anchor;

                trace!(
                    "-> {} glyphs, w={} h={} scale={} alignment_translation={:?}",
                    text_layout_info.glyphs.len(),
                    width,
                    height,
                    scale_factor,
                    alignment_translation
                );

                let mut extracted_glyphs = vec![];
                for text_glyph in &text_layout_info.glyphs {
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
                    let handle = text_glyph.atlas_info.texture.clone_weak();
                    let index = text_glyph.atlas_info.glyph_index as usize;
                    let uv_rect = atlas.textures[index];

                    let glyph_offset = alignment_translation + text_glyph.position;

                    extracted_glyphs.push(ExtractedGlyph {
                        offset: glyph_offset,
                        size: text_glyph.size,
                        color,
                        handle_id: handle.id(),
                        uv_rect,
                    });

                    // let glyph_transform = Transform::from_translation(
                    //     alignment_offset * scale_factor +
                    // text_glyph.position.extend(0.), );

                    // let transform =
                    // text_transform.mul_transform(glyph_transform);

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
        let extracted_canvas = extracted_canvases
            .entry(entity)
            .or_insert(ExtractedCanvas::default());
        extracted_canvas.transform = *transform;
        extracted_canvas.primitives = primitives;
        extracted_canvas.scale_factor = scale_factor;
        extracted_canvas.texts = extracted_texts;
    }
}

/// Iterator over sub-primitives of a primitive.
pub(crate) struct SubPrimIter<'a> {
    /// The current primitive being iterated over, or `None` if the iterator
    /// reached the end of the iteration sequence.
    prim: Option<Primitive>,
    /// The index of the current sub-primitive inside its parent primitive.
    index: usize,
    /// Text information for iterating over glyphs.
    texts: &'a [ExtractedText],
}

impl<'a> SubPrimIter<'a> {
    pub fn new(prim: Primitive, texts: &'a [ExtractedText]) -> Self {
        Self {
            prim: Some(prim),
            index: 0,
            texts,
        }
    }
}

impl<'a> Iterator for SubPrimIter<'a> {
    type Item = (AssetId<Image>, u32);

    fn next(&mut self) -> Option<Self::Item> {
        if let Some(prim) = &self.prim {
            let PrimitiveInfo {
                row_count: _,
                index_count,
            } = prim.info(self.texts);
            match prim {
                Primitive::Text(text) => {
                    if text.id as usize >= self.texts.len() {
                        return None; // not ready
                    }
                    let text = &self.texts[text.id as usize];
                    if self.index < text.glyphs.len() {
                        let image_handle_id = text.glyphs[self.index].handle_id;
                        self.index += 1;
                        Some((image_handle_id, TextPrimitive::INDEX_PER_GLYPH))
                    } else {
                        self.prim = None;
                        None
                    }
                }
                Primitive::Rect(rect) => {
                    let handle_id = if let Some(id) = rect.image {
                        id
                    } else {
                        AssetId::<Image>::invalid()
                    };
                    self.prim = None;
                    Some((handle_id, index_count))
                }
                _ => {
                    self.prim = None;
                    // Currently all other primitives are non-textured
                    Some((AssetId::<Image>::invalid(), index_count))
                }
            }
        } else {
            None
        }
    }
}

/// Format a list of values as 16 values per row, for more compact `trace!()`.
///
/// ```ignore
/// trace_list!("x = ", my_iter, " {}");
/// ```
macro_rules! trace_list {
    ($header:expr, $iter:expr, $fmt:expr) => {
        if enabled!(bevy::log::Level::TRACE) {
            let mut s = String::with_capacity(256);
            for u in $iter.chunks(16) {
                s.clear();
                s += $header;
                u.iter().fold(&mut s, |s, u| {
                    write!(s, $fmt, u).unwrap();
                    s
                });
                trace!("{}", s);
            }
        }
    };
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
            AssetEvent::Added { .. } | AssetEvent::LoadedWithDependencies { .. } => None,
            AssetEvent::Modified { id }
            | AssetEvent::Removed { id }
            | AssetEvent::Unused { id } => {
                let removed = image_bind_groups.values.remove(id);
                if removed.is_some() {
                    debug!("Removed IBG for handle {:?} due to {:?}", id, event);
                }
                removed
            }
        };
    }

    let extracted_canvases = &mut extracted_canvases.canvases;

    for (entity, extracted_canvas) in extracted_canvases {
        trace!(
            "Canvas on Entity {:?} has {} primitives and {} texts",
            entity,
            extracted_canvas.primitives.len(),
            extracted_canvas.texts.len(),
        );

        let mut primitives = vec![];
        let mut indices = vec![];

        // Serialize primitives into a binary float32 array, to work around the fact
        // wgpu doesn't have byte arrays. And f32 being the most common type of
        // data in primitives limits the amount of bitcast in the shader.
        trace!(
            "Serialize {} primitives...",
            extracted_canvas.primitives.len()
        );
        let mut current_batch = PrimitiveBatch::invalid();
        for prim in &extracted_canvas.primitives {
            let base_index = primitives.len() as u32;
            trace!("+ Primitive @ base_index={}", base_index);

            // Serialize the primitive
            let PrimitiveInfo {
                row_count,
                index_count,
            } = prim.info(&extracted_canvas.texts[..]);
            trace!("=> rs={} is={}", row_count, index_count);
            if row_count > 0 && index_count > 0 {
                let row_count = row_count as usize;
                let index_count = index_count as usize;

                // Reserve some (uninitialized) storage for new data
                primitives.reserve(row_count);
                indices.reserve(index_count);
                let prim_slice = primitives.spare_capacity_mut();
                let idx_slice = indices.spare_capacity_mut();

                // Write primitives and indices directly into storage
                prim.write(
                    &extracted_canvas.texts[..],
                    &mut prim_slice[..row_count],
                    base_index,
                    &mut idx_slice[..index_count],
                    extracted_canvas.scale_factor,
                );

                // Apply new storage sizes once data is initialized
                let new_row_count = primitives.len() + row_count;
                unsafe { primitives.set_len(new_row_count) };
                let new_index_count = indices.len() + index_count;
                unsafe { indices.set_len(new_index_count) };

                trace!("New primitive elements: (+{})", row_count);
                trace_list!(
                    "+ f32[] =",
                    primitives[new_row_count - row_count..new_row_count],
                    " {}"
                );
                trace!("New indices: (+{})", index_count);
                trace_list!(
                    "+ u32[] =",
                    indices[new_index_count - index_count..new_index_count],
                    " {:x}"
                );
            }

            // Loop on sub-primitives; Text primitives expand to one Rect primitive
            // per glyph, each of which _can_ have a separate atlas texture so potentially
            // can split the draw into a new batch.
            trace!("Batch sub-primitives...");
            let batch_iter = SubPrimIter::new(*prim, &extracted_canvas.texts);
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
                if current_batch.image_handle_id == AssetId::<Image>::invalid() {
                    trace!(
                        "Spawning batch: canvas_entity={:?} index={:?} (no tex)",
                        current_batch.canvas_entity,
                        current_batch.range,
                    );
                    commands.spawn(current_batch);
                } else if let Some(gpu_image) = gpu_images.get(current_batch.image_handle_id) {
                    image_bind_groups
                        .values
                        .entry(current_batch.image_handle_id)
                        .or_insert_with(|| {
                            debug!(
                                "Insert new bind group for handle={:?}",
                                current_batch.image_handle_id
                            );
                            render_device.create_bind_group(
                                "primitive_material_bind_group",
                                &primitive_pipeline.material_layout,
                                &[
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
                            )
                        });

                    trace!(
                        "Spawning batch: canvas_entity={:?} index={:?} handle={:?}",
                        current_batch.canvas_entity,
                        current_batch.range,
                        current_batch.image_handle_id,
                    );
                    commands.spawn(current_batch);
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
        trace!("Output last batch...");
        if !current_batch.range.is_empty() {
            // Check if the previous batch image is available on GPU; if so output the batch
            if current_batch.image_handle_id == AssetId::<Image>::invalid() {
                trace!(
                    "Spawning batch: canvas_entity={:?} index={:?} (no tex)",
                    current_batch.canvas_entity,
                    current_batch.range,
                );
                commands.spawn(current_batch);
            } else if let Some(gpu_image) = gpu_images.get(current_batch.image_handle_id) {
                image_bind_groups
                    .values
                    .entry(current_batch.image_handle_id)
                    .or_insert_with(|| {
                        trace!(
                            "Insert new bind group for handle={:?}",
                            current_batch.image_handle_id
                        );
                        render_device.create_bind_group(
                            "primitive_material_bind_group",
                            &primitive_pipeline.material_layout,
                            &[
                                BindGroupEntry {
                                    binding: 0,
                                    resource: BindingResource::TextureView(&gpu_image.texture_view),
                                },
                                BindGroupEntry {
                                    binding: 1,
                                    resource: BindingResource::Sampler(&gpu_image.sampler),
                                },
                            ],
                        )
                    });

                trace!(
                    "Spawning batch: canvas_entity={:?} index={:?} handle={:?}",
                    current_batch.canvas_entity,
                    current_batch.range,
                    current_batch.image_handle_id,
                );
                commands.spawn(current_batch);
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
    extracted_canvases: Res<ExtractedCanvases>,
    mut views: Query<&mut RenderPhase<Transparent2d>>,
    batches: Query<(Entity, &PrimitiveBatch)>,
) {
    trace!("queue_primitives: {} batches", batches.iter().len());

    let view_binding = match view_uniforms.uniforms.binding() {
        Some(view_binding) => view_binding,
        None => {
            return;
        }
    };

    let primitive_meta = &mut primitive_meta;

    primitive_meta.view_bind_group = Some(render_device.create_bind_group(
        "primitive_view_bind_group",
        &primitive_pipeline.view_layout,
        &[BindGroupEntry {
            binding: 0,
            resource: view_binding,
        }],
    ));

    // TODO - per view culling?! (via VisibleEntities)
    trace!("Specializing pipeline(s)...");
    let draw_primitives_function = draw_functions.read().get_id::<DrawPrimitive>().unwrap();
    let key = PrimitivePipelineKey::from_msaa_samples(msaa.samples());
    let untextured_pipeline = pipelines.specialize(&mut pipeline_cache, &primitive_pipeline, key);
    let textured_pipeline = pipelines.specialize(
        &mut pipeline_cache,
        &primitive_pipeline,
        key | PrimitivePipelineKey::TEXTURED,
    );

    trace!("Looping on batches...");
    for (batch_entity, batch) in batches.iter() {
        trace!("batch ent={:?} range={:?}", batch_entity, batch.range);
        if batch.range.is_empty() {
            continue;
        }

        let canvas_entity = batch.canvas_entity;

        let is_textured = batch.image_handle_id != AssetId::<Image>::invalid();
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

        let primitive_bind_group = render_device.create_bind_group(
            "prim_bind_group",
            &primitive_pipeline.prim_layout,
            &[BindGroupEntry {
                binding: 0,
                resource: extracted_canvas.binding().unwrap(),
            }],
        );
        let index_buffer = extracted_canvas.index_buffer.as_ref().unwrap().clone();

        // Update meta map
        primitive_meta
            .canvas_meta
            .entry(canvas_entity)
            .and_modify(|canvas_meta| {
                // canvas_meta.batch_entity = batch_entity;
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
                "Add Transparent2d entity={:?} range={:?} (sort={:?})",
                batch_entity,
                batch.range,
                sort_key
            );
            transparent_phase.add(Transparent2d {
                draw_function: draw_primitives_function,
                pipeline,
                entity: batch_entity,
                sort_key,
                // This is batching multiple items into a single draw call, which is not a feature
                // of bevy_render we currently use
                batch_range: 0..1,
                dynamic_offset: None,
            });
        }
    }
}
