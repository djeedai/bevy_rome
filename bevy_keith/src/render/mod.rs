use std::fmt::Write as _;

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
    math::bounding::{Aabb2d, IntersectsVolume},
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
            FragmentState, FrontFace, MultisampleState, PipelineCache, PolygonMode, PrimitiveState,
            PrimitiveTopology, RenderPipelineDescriptor, SamplerBindingType, ShaderStages,
            ShaderType, SpecializedRenderPipeline, SpecializedRenderPipelines, TextureFormat,
            TextureSampleType, TextureViewDimension, VertexState,
        },
        renderer::{RenderDevice, RenderQueue},
        texture::{BevyDefault, FallbackImage, Image},
        view::{Msaa, ViewUniform, ViewUniformOffset, ViewUniforms},
        Extract,
    },
    utils::{tracing::enabled, FloatOrd, HashMap},
    window::PrimaryWindow,
};

use crate::{
    canvas::{
        Canvas, OffsetAndCount, PrimImpl, Primitive, PrimitiveIndexAndKind, PrimitiveInfo, Tiles,
    },
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
        let image_bind_groups = image_bind_groups.into_inner();
        if primitive_batch.image_handle_id != AssetId::<Image>::invalid() {
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
            trace!("image_bind_groups:");
            for (handle, bind_group) in &image_bind_groups.values {
                trace!("+ ibg: {:?} = {:?}", handle, bind_group);
            }
            let Some(ibg) = image_bind_groups
                .values
                .get(&primitive_batch.image_handle_id)
            else {
                error!("Failed to find IBG!");
                return RenderCommandResult::Failure;
            };
            pass.set_bind_group(I, ibg, &[]);
        } else if let Some(ibg) = image_bind_groups.fallback.as_ref() {
            // We need a texture anyway, bind anything to make the shader happy
            pass.set_bind_group(I, ibg, &[]);
        } else {
            // We can't use this shader without a valid bind group
            return RenderCommandResult::Failure;
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

        if let Some(_canvas_meta) = primitive_meta
            .canvas_meta
            .get(&primitive_batch.canvas_entity)
        {
            // Draw a single fullscreen triangle, implicitly defined by its vertex IDs
            trace!("DrawPrimitiveBatch");
            pass.draw(0..3, 0..1);
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
        }
    }

    pub fn is_valid(&self) -> bool {
        self.canvas_entity != Entity::PLACEHOLDER
    }

    /// Try to merge a batch into the current batch.
    ///
    /// Return `true` if the batch was merged, or `false` otherwise.
    pub fn try_merge(&mut self, other: &PrimitiveBatch) -> bool {
        if self.is_handle_compatible(other.image_handle_id)
            && self.canvas_entity == other.canvas_entity
        {
            // Overwrite in case self is invalid
            if self.image_handle_id == AssetId::invalid() {
                self.image_handle_id = other.image_handle_id;
            }
            true
        } else {
            false
        }
    }

    fn is_handle_compatible(&self, handle: AssetId<Image>) -> bool {
        // Any invalid handle means "no texture", which can be batched with any other
        // texture. Only different (valid) textures cannot be batched together.
        return handle == AssetId::invalid()
            || self.image_handle_id == AssetId::invalid()
            || self.image_handle_id == handle;
    }
}

/// Metadata for [`Canvas`] rendering.
struct CanvasMeta {
    /// Entity the [`Canvas`] component is attached to.
    canvas_entity: Entity,
    /// Bind group for the primitive buffer used by the canvas.
    primitive_bind_group: BindGroup,
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
    fallback: Option<BindGroup>,
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
            "keith:canvas_view_layout",
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
            "keith:canvas_prim_layout",
            &[
                BindGroupLayoutEntry {
                    binding: 0,
                    visibility: ShaderStages::VERTEX | ShaderStages::FRAGMENT,
                    ty: BindingType::Buffer {
                        ty: BufferBindingType::Storage { read_only: true },
                        has_dynamic_offset: false,
                        min_binding_size: BufferSize::new(4_u64), // f32
                    },
                    count: None,
                },
                BindGroupLayoutEntry {
                    binding: 1,
                    visibility: ShaderStages::VERTEX | ShaderStages::FRAGMENT,
                    ty: BindingType::Buffer {
                        ty: BufferBindingType::Storage { read_only: true },
                        has_dynamic_offset: false,
                        min_binding_size: BufferSize::new(4_u64), // u32
                    },
                    count: None,
                },
                BindGroupLayoutEntry {
                    binding: 2,
                    visibility: ShaderStages::VERTEX | ShaderStages::FRAGMENT,
                    ty: BindingType::Buffer {
                        ty: BufferBindingType::Storage { read_only: true },
                        has_dynamic_offset: false,
                        min_binding_size: BufferSize::new(8_u64), // u32 * 2
                    },
                    count: None,
                },
            ],
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
        const NONE               = 0;
        const MSAA_RESERVED_BITS = PrimitivePipelineKey::MSAA_MASK_BITS << PrimitivePipelineKey::MSAA_SHIFT_BITS;
    }
}

impl PrimitivePipelineKey {
    const MSAA_MASK_BITS: u32 = 0b111111;
    const MSAA_SHIFT_BITS: u32 = 32 - 6;

    pub fn from_msaa_samples(msaa_samples: u32) -> Self {
        assert!(msaa_samples > 0);
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
        RenderPipelineDescriptor {
            vertex: VertexState {
                shader: PRIMITIVE_SHADER_HANDLE,
                entry_point: "vertex".into(),
                shader_defs: vec![],
                buffers: vec![], // vertex-less rendering
            },
            fragment: Some(FragmentState {
                shader: PRIMITIVE_SHADER_HANDLE,
                shader_defs: vec![],
                entry_point: "fragment".into(),
                targets: vec![Some(ColorTargetState {
                    format: TextureFormat::bevy_default(),
                    blend: Some(BlendState::ALPHA_BLENDING),
                    write_mask: ColorWrites::ALL,
                })],
            }),
            layout: vec![
                self.view_layout.clone(),
                self.prim_layout.clone(),
                self.material_layout.clone(),
            ],
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
            label: Some("keith:primitive_pipeline".into()),
            push_constant_ranges: vec![],
        }
    }
}

/// Rendering data extracted from a single [`Canvas`] component.
#[derive(Default)]
pub struct ExtractedCanvas {
    /// Global transform of the canvas.
    pub transform: GlobalTransform,
    pub screen_size: UVec2,
    pub canvas_origin: Vec2,
    /// Canvas rectangle relative to its origin.
    pub rect: Rect,
    /// Collection of primitives rendered in this canvas.
    pub primitives: Vec<Primitive>,
    storage: Option<Buffer>,
    storage_capacity: usize,
    tile_primitives_buffer: Option<Buffer>,
    tile_primitives_buffer_capacity: usize,
    offset_and_count_buffer: Option<Buffer>,
    offset_and_count_buffer_capacity: usize,
    /// Scale factor of the window where this canvas is rendered.
    pub scale_factor: f32,
    /// Extracted data for all texts in use, in local text ID order.
    pub(crate) texts: Vec<ExtractedText>,
    pub(crate) tiles: Tiles,
}

impl ExtractedCanvas {
    /// Write the CPU scratch buffer into the associated GPU storage buffer.
    pub fn write_buffers(
        &mut self,
        primitives: &[f32],
        render_device: &RenderDevice,
        render_queue: &RenderQueue,
    ) {
        trace!(
            "Writing {} primitive elements to GPU buffers",
            primitives.len(),
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
                    label: Some("keith:canvas_primitive_buffer"),
                    usage: BufferUsages::COPY_DST | BufferUsages::STORAGE,
                    contents,
                }),
            );
            self.storage_capacity = size;
        } else if let Some(storage) = &self.storage {
            // Write directly to existing GPU buffer
            render_queue.write_buffer(storage, 0, contents);
        }

        // Tile primitives buffer
        let size = self.tiles.primitives.len(); // FIXME - cap size to reasonable value
        let contents = bytemuck::cast_slice(&self.tiles.primitives[..]);
        if size > self.tile_primitives_buffer_capacity {
            // GPU buffer too small; reallocated...
            trace!(
                "Reallocate canvas_tile_primitive_buffer: {} -> {}",
                self.tile_primitives_buffer_capacity,
                size
            );
            self.tile_primitives_buffer = Some(render_device.create_buffer_with_data(
                &BufferInitDescriptor {
                    label: Some("keith:canvas_tile_primitive_buffer"),
                    usage: BufferUsages::COPY_DST | BufferUsages::STORAGE,
                    contents,
                },
            ));
            self.tile_primitives_buffer_capacity = size;
        } else if let Some(tile_primitives_buffer) = &self.tile_primitives_buffer {
            // Write directly to existing GPU buffer
            render_queue.write_buffer(tile_primitives_buffer, 0, contents);
        }

        // Offset and count buffer
        let size = self.tiles.offset_and_count.len() * 2; // FIXME - cap size to reasonable value
        let contents = bytemuck::cast_slice(&self.tiles.offset_and_count[..]);
        if size > self.offset_and_count_buffer_capacity {
            // GPU buffer too small; reallocated...
            trace!(
                "Reallocate canvas_offset_and_count_buffer: {} -> {}",
                self.offset_and_count_buffer_capacity,
                size
            );
            self.offset_and_count_buffer = Some(render_device.create_buffer_with_data(
                &BufferInitDescriptor {
                    label: Some("keith:canvas_offset_and_count_buffer"),
                    usage: BufferUsages::COPY_DST | BufferUsages::STORAGE,
                    contents,
                },
            ));
            self.offset_and_count_buffer_capacity = size;
        } else if let Some(offset_and_count_buffer) = &self.offset_and_count_buffer {
            // Write directly to existing GPU buffer
            render_queue.write_buffer(offset_and_count_buffer, 0, contents);
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

    #[inline]
    pub fn tile_primitives_binding(&self) -> Option<BindingResource> {
        self.tile_primitives_buffer.as_ref().map(|buffer| {
            BindingResource::Buffer(BufferBinding {
                buffer: &buffer,
                offset: 0,
                size: None,
            })
        })
    }

    #[inline]
    pub fn offset_and_count_binding(&self) -> Option<BindingResource> {
        self.offset_and_count_buffer.as_ref().map(|buffer| {
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
    canvas_query: Extract<
        Query<(
            Entity,
            Option<&ViewVisibility>,
            &Camera,
            &OrthographicProjection,
            &Canvas,
            &GlobalTransform,
            &Tiles,
        )>,
    >,
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

    for (entity, maybe_computed_visibility, camera, proj, canvas, transform, tiles) in
        canvas_query.iter()
    {
        // Skip hidden canvases. If no ComputedVisibility component is present, assume
        // visible.
        if !maybe_computed_visibility.map_or(true, |cvis| cvis.get()) {
            continue;
        }

        // Get screen size of camera, to calculate number of tiles to allocate
        let Some(screen_size) = camera.physical_viewport_size() else {
            continue;
        };

        // Swap render and main app primitive buffer
        let primitives = canvas.buffer().clone();
        trace!(
            "Canvas on Entity {:?} has {} primitives and {} text layouts, viewport_origin={:?}, viewport_area={:?}, scale_factor={}, proj.scale={}",
            entity,
            primitives.len(),
            canvas.text_layouts().len(),
            proj.viewport_origin,
            proj.area,
            scale_factor,
            proj.scale
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
        extracted_canvas.screen_size = screen_size;
        extracted_canvas.canvas_origin = -proj.area.min * scale_factor; // in physical pixels
        extracted_canvas.rect = canvas.rect();
        extracted_canvas.primitives = primitives;
        extracted_canvas.scale_factor = scale_factor;
        extracted_canvas.texts = extracted_texts;
        extracted_canvas.tiles = tiles.clone();
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
    type Item = AssetId<Image>;

    fn next(&mut self) -> Option<Self::Item> {
        if let Some(prim) = &self.prim {
            let PrimitiveInfo { row_count: _ } = prim.info(self.texts);
            match prim {
                Primitive::Text(text) => {
                    if text.id as usize >= self.texts.len() {
                        return None; // not ready
                    }
                    let text = &self.texts[text.id as usize];
                    if self.index < text.glyphs.len() {
                        let image_handle_id = text.glyphs[self.index].handle_id;
                        self.index += 1;
                        Some(image_handle_id)
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
                    Some(handle_id)
                }
                _ => {
                    self.prim = None;
                    // Currently all other primitives are non-textured
                    Some(AssetId::<Image>::invalid())
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

pub(crate) struct PreparedPrimitive {
    /// AABB in canvas space, for tile assignment.
    pub aabb: Aabb2d,
    /// Primitive index.
    pub prim_index: PrimitiveIndexAndKind,
}

pub(crate) fn prepare_primitives(
    mut commands: Commands,
    mut extracted_canvases: ResMut<ExtractedCanvases>,
    render_device: Res<RenderDevice>,
    render_queue: Res<RenderQueue>,
    mut image_bind_groups: ResMut<ImageBindGroups>,
    events: Res<PrimitiveAssetEvents>,
    mut prepared_primitives: Local<Vec<PreparedPrimitive>>,
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

    // Loop on all extracted canvases to process their primitives
    for (entity, extracted_canvas) in extracted_canvases {
        trace!(
            "Canvas on Entity {:?} has {} primitives and {} texts, tile size {:?}, canvas_origin={:?}",
            entity,
            extracted_canvas.primitives.len(),
            extracted_canvas.texts.len(),
            extracted_canvas.tiles.tile_size,
            extracted_canvas.canvas_origin,
        );

        let mut primitives = vec![];

        prepared_primitives.clear();
        prepared_primitives.reserve(extracted_canvas.primitives.len());

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
            let is_textured = prim.is_textured();
            let prim_index = PrimitiveIndexAndKind::new(base_index, prim.gpu_kind(), is_textured);

            // Calculate once and save the AABB of the primitive, for tile assignment
            // purpose. Since there are many more tiles than primitives, it's worth doing
            // that calculation only once ahead of time before looping over tiles.
            let mut aabb = prim.aabb();
            aabb.min += extracted_canvas.canvas_origin;
            aabb.max += extracted_canvas.canvas_origin;
            prepared_primitives.push(PreparedPrimitive { aabb, prim_index });

            trace!("+ Primitive @ base_index={} aabb={:?}", base_index, aabb);

            // Serialize the primitive
            let PrimitiveInfo { row_count } = prim.info(&extracted_canvas.texts[..]);
            trace!("  row_count={}", row_count);
            if row_count > 0 {
                let row_count = row_count as usize;

                // Reserve some (uninitialized) storage for new data
                primitives.reserve(row_count);
                let prim_slice = primitives.spare_capacity_mut();

                // Write primitives and indices directly into storage
                prim.write(
                    &extracted_canvas.texts[..],
                    &mut prim_slice[..row_count],
                    extracted_canvas.scale_factor,
                );

                // Apply new storage sizes once data is initialized
                let new_row_count = primitives.len() + row_count;
                unsafe { primitives.set_len(new_row_count) };

                trace!("New primitive elements: (+{})", row_count);
                trace_list!(
                    "+ f32[] =",
                    primitives[new_row_count - row_count..new_row_count],
                    " {}"
                );
            }

            // Loop on sub-primitives; Text primitives expand to one Rect primitive
            // per glyph, each of which _can_ have a separate atlas texture so potentially
            // can split the draw into a new batch.
            trace!("Batch sub-primitives...");
            let batch_iter = SubPrimIter::new(*prim, &extracted_canvas.texts);
            for image_handle_id in batch_iter {
                let new_batch = PrimitiveBatch {
                    image_handle_id,
                    canvas_entity: *entity,
                };
                trace!(
                    "New Batch: canvas_entity={:?} image={:?}",
                    new_batch.canvas_entity,
                    new_batch.image_handle_id
                );

                if current_batch.try_merge(&new_batch) {
                    trace!(
                        "Merged new batch with current batch: image={:?}",
                        current_batch.image_handle_id
                    );
                    continue;
                }

                // Batches are different; output the previous one before starting a new one.

                // Skip if batch is empty, which may happen on first one (current_batch
                // initialized to an invalid empty batch)
                if current_batch.is_valid() {
                    commands.spawn(current_batch);
                }

                current_batch = new_batch;
            }
        }

        // Output the last batch
        if current_batch.is_valid() {
            trace!("Output last batch...");
            commands.spawn(current_batch);
        }

        if primitives.is_empty() {
            trace!("No primitive to render, finished preparing.");
            return;
        }

        // Assign primitives to tiles
        let tile_size = extracted_canvas.tiles.tile_size.as_vec2();
        for ty in 0..extracted_canvas.tiles.dimensions.y {
            for tx in 0..extracted_canvas.tiles.dimensions.x {
                let min = Vec2::new(tx as f32, ty as f32) * tile_size;
                let max = min + tile_size;
                let tile_aabb = Aabb2d { min, max };

                let offset = extracted_canvas.tiles.primitives.len() as u32;

                // Loop on all primitives to gather the ones affecting the current tile. We
                // expect a lot more tiles than primitives for a standard 1080p or 4K screen
                // resolution.
                let mut count = 0;
                for prim in &prepared_primitives {
                    if prim.aabb.intersects(&tile_aabb) {
                        // trace!("Prim #{count} base_index={base_index} aabb={prim_aabb:?}
                        // overlaps tile {tx}x{ty} with aabb {tile_aabb:?}");
                        extracted_canvas.tiles.primitives.push(prim.prim_index);
                        count += 1;
                    }
                }

                extracted_canvas
                    .tiles
                    .offset_and_count
                    .push(OffsetAndCount { offset, count });
            }
        }

        // Write to GPU buffers
        trace!(
            "Writing {} elems for Canvas of entity {:?}",
            primitives.len(),
            entity
        );
        extracted_canvas.write_buffers(&primitives[..], &render_device, &render_queue);
    }
}

#[allow(clippy::too_many_arguments)]
pub fn queue_primitives(
    draw_functions: Res<DrawFunctions<Transparent2d>>,
    primitive_pipeline: Res<PrimitivePipeline>,
    mut pipelines: ResMut<SpecializedRenderPipelines<PrimitivePipeline>>,
    mut pipeline_cache: ResMut<PipelineCache>,
    msaa: Res<Msaa>,
    extracted_canvases: Res<ExtractedCanvases>,
    mut views: Query<&mut RenderPhase<Transparent2d>>,
    batches: Query<(Entity, &PrimitiveBatch)>,
) {
    trace!("queue_primitives: {} batches", batches.iter().len());

    // TODO - per view culling?! (via VisibleEntities)
    trace!("Specializing pipeline(s)...");
    let draw_primitives_function = draw_functions.read().get_id::<DrawPrimitive>().unwrap();
    let key = PrimitivePipelineKey::from_msaa_samples(msaa.samples());
    let primitive_pipeline = pipelines.specialize(&mut pipeline_cache, &primitive_pipeline, key);
    trace!("primitive_pipeline={:?}", primitive_pipeline,);

    trace!("Looping on batches...");
    for (batch_entity, batch) in batches.iter() {
        trace!(
            "batch ent={:?} image={:?}",
            batch_entity,
            batch.image_handle_id
        );
        if !batch.is_valid() {
            // shouldn't happen
            continue;
        }

        let canvas_entity = batch.canvas_entity;

        let is_textured = batch.image_handle_id != AssetId::<Image>::invalid();
        trace!("  is_textured={}", is_textured);

        let extracted_canvas =
            if let Some(extracted_canvas) = extracted_canvases.canvases.get(&canvas_entity) {
                extracted_canvas
            } else {
                continue;
            };

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
                "Add Transparent2d entity={:?} image={:?} pipeline={:?} (sort={:?})",
                batch_entity,
                batch.image_handle_id,
                primitive_pipeline,
                sort_key
            );
            transparent_phase.add(Transparent2d {
                draw_function: draw_primitives_function,
                pipeline: primitive_pipeline,
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

pub fn prepare_bind_groups(
    render_device: Res<RenderDevice>,
    view_uniforms: Res<ViewUniforms>,
    primitive_pipeline: Res<PrimitivePipeline>,
    batches: Query<(Entity, &PrimitiveBatch)>,
    extracted_canvases: Res<ExtractedCanvases>,
    gpu_images: Res<RenderAssets<Image>>,
    fallback_images: Res<FallbackImage>,
    mut primitive_meta: ResMut<PrimitiveMeta>,
    mut image_bind_groups: ResMut<ImageBindGroups>,
) {
    trace!("prepare_bind_groups()");

    let Some(view_binding) = view_uniforms.uniforms.binding() else {
        trace!("View binding not available; aborted.");
        return;
    };

    if image_bind_groups.fallback.is_none() {
        image_bind_groups.fallback = Some(render_device.create_bind_group(
            "keith:fallback_primitive_material_bind_group",
            &primitive_pipeline.material_layout,
            &[
                BindGroupEntry {
                    binding: 0,
                    resource: BindingResource::TextureView(&fallback_images.d2.texture_view),
                },
                BindGroupEntry {
                    binding: 1,
                    resource: BindingResource::Sampler(&fallback_images.d2.sampler),
                },
            ],
        ));
        debug!(
            "Created bind group for fallback primitive texture: {:?}",
            image_bind_groups.fallback.as_ref().unwrap()
        );
    }

    primitive_meta.view_bind_group = Some(render_device.create_bind_group(
        "keith:primitive_view_bind_group",
        &primitive_pipeline.view_layout,
        &[BindGroupEntry {
            binding: 0,
            resource: view_binding,
        }],
    ));

    trace!("Looping on batches...");
    for (batch_entity, batch) in batches.iter() {
        trace!(
            "batch ent={:?} image={:?}",
            batch_entity,
            batch.image_handle_id
        );
        if !batch.is_valid() {
            // shouldn't happen
            continue;
        }

        let canvas_entity = batch.canvas_entity;

        let extracted_canvas =
            if let Some(extracted_canvas) = extracted_canvases.canvases.get(&canvas_entity) {
                extracted_canvas
            } else {
                continue;
            };

        let (Some(prim), Some(tile_prim), Some(oc)) = (
            extracted_canvas.binding(),
            extracted_canvas.tile_primitives_binding(),
            extracted_canvas.offset_and_count_binding(),
        ) else {
            continue;
        };

        let primitive_bind_group = render_device.create_bind_group(
            Some(&format!("keith:prim_bind_group_{:?}", canvas_entity)[..]),
            &primitive_pipeline.prim_layout,
            &[
                BindGroupEntry {
                    binding: 0,
                    resource: prim,
                },
                BindGroupEntry {
                    binding: 1,
                    resource: tile_prim,
                },
                BindGroupEntry {
                    binding: 2,
                    resource: oc,
                },
            ],
        );

        // Update meta map
        primitive_meta
            .canvas_meta
            .entry(canvas_entity)
            .and_modify(|canvas_meta| {
                // Overwrite bind groups; any old one might reference a deallocated buffer
                canvas_meta.primitive_bind_group = primitive_bind_group.clone();
            })
            .or_insert_with(|| {
                trace!("Adding new CanvasMeta: canvas_entity={:?}", canvas_entity);
                CanvasMeta {
                    canvas_entity,
                    primitive_bind_group,
                }
            });

        trace!(
            "CanvasMeta: canvas_entity={:?} batch_entity={:?}",
            canvas_entity,
            batch_entity,
        );

        // Set bind group for texture, if any
        if batch.image_handle_id != AssetId::<Image>::invalid() {
            if let Some(gpu_image) = gpu_images.get(batch.image_handle_id) {
                image_bind_groups
                    .values
                    .entry(batch.image_handle_id)
                    .or_insert_with(|| {
                        debug!(
                            "Insert new bind group for handle={:?}",
                            batch.image_handle_id
                        );
                        render_device.create_bind_group(
                            "keith:primitive_material_bind_group",
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
            } else {
                warn!(
                    "GPU image for asset {:?} is not available, cannot create bind group!",
                    batch.image_handle_id
                );
            }
        }
    }
}
