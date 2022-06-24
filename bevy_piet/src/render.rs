use bevy::{
    asset::{AssetEvent, Handle, HandleId},
    core::{FloatOrd, Pod, Zeroable},
    core_pipeline::Transparent2d,
    ecs::{
        component::Component,
        entity::Entity,
        system::{
            lifetimeless::{Read, SQuery, SRes},
            Commands, Query, Res, ResMut, SystemParamItem,
        },
        world::{FromWorld, World},
    },
    math::const_vec2,
    prelude::*,
    reflect::Uuid,
    render::{
        render_asset::RenderAssets,
        render_phase::{
            BatchedPhaseItem, DrawFunctions, EntityRenderCommand, RenderCommand,
            RenderCommandResult, RenderPhase, SetItemPipeline, TrackedRenderPass,
        },
        render_resource::{
            std140::AsStd140, BindGroup, BindGroupDescriptor, BindGroupEntry, BindGroupLayout,
            BindGroupLayoutDescriptor, BindGroupLayoutEntry, BindingResource, BindingType,
            BlendState, BufferBindingType, BufferSize, BufferUsages, BufferVec, ColorTargetState,
            ColorWrites, FragmentState, FrontFace, MultisampleState, PipelineCache, PolygonMode,
            PrimitiveState, PrimitiveTopology, RenderPipelineDescriptor, SamplerBindingType,
            ShaderStages, SpecializedRenderPipeline, SpecializedRenderPipelines, TextureFormat,
            TextureSampleType, TextureViewDimension, VertexBufferLayout, VertexFormat, VertexState,
            VertexStepMode,
        },
        renderer::{RenderDevice, RenderQueue},
        texture::{BevyDefault, Image},
        view::{Msaa, ViewUniform, ViewUniformOffset, ViewUniforms},
        RenderWorld,
    },
    utils::HashMap,
};
use copyless::VecHelper;

use crate::{PietCanvas, QUAD_SHADER_HANDLE};

pub type DrawPiet = (
    SetItemPipeline,
    SetQuadViewBindGroup<0>,
    SetQuadTextureBindGroup<1>,
    DrawQuadBatch,
);

pub struct SetQuadViewBindGroup<const I: usize>;

impl<const I: usize> EntityRenderCommand for SetQuadViewBindGroup<I> {
    type Param = (SRes<QuadMeta>, SQuery<Read<ViewUniformOffset>>);

    fn render<'w>(
        view: Entity,
        _item: Entity,
        (quad_meta, view_query): SystemParamItem<'w, '_, Self::Param>,
        pass: &mut TrackedRenderPass<'w>,
    ) -> RenderCommandResult {
        let view_uniform = view_query.get(view).unwrap();
        pass.set_bind_group(
            I,
            quad_meta.into_inner().view_bind_group.as_ref().unwrap(),
            &[view_uniform.offset],
        );
        RenderCommandResult::Success
    }
}

pub struct SetQuadTextureBindGroup<const I: usize>;

impl<const I: usize> EntityRenderCommand for SetQuadTextureBindGroup<I> {
    type Param = (SRes<ImageBindGroups>, SQuery<Read<QuadBatch>>);

    fn render<'w>(
        _view: Entity,
        item: Entity,
        (image_bind_groups, query_batch): SystemParamItem<'w, '_, Self::Param>,
        pass: &mut TrackedRenderPass<'w>,
    ) -> RenderCommandResult {
        let quad_batch = query_batch.get(item).unwrap();
        if quad_batch.textured {
            let image_bind_groups = image_bind_groups.into_inner();
            pass.set_bind_group(
                I,
                image_bind_groups
                    .values
                    .get(&Handle::weak(quad_batch.image_handle_id))
                    .unwrap(),
                &[],
            );
        }
        RenderCommandResult::Success
    }
}

pub struct DrawQuadBatch;

impl<P: BatchedPhaseItem> RenderCommand<P> for DrawQuadBatch {
    type Param = (SRes<QuadMeta>, SQuery<Read<QuadBatch>>);

    fn render<'w>(
        _view: Entity,
        item: &P,
        (quad_meta, query_batch): SystemParamItem<'w, '_, Self::Param>,
        pass: &mut TrackedRenderPass<'w>,
    ) -> RenderCommandResult {
        let quad_batch = query_batch.get(item.entity()).unwrap();
        let quad_meta = quad_meta.into_inner();
        if quad_batch.textured {
            pass.set_vertex_buffer(0, quad_meta.textured_vertices.buffer().unwrap().slice(..));
        } else {
            pass.set_vertex_buffer(0, quad_meta.vertices.buffer().unwrap().slice(..));
        }
        pass.draw(item.batch_range().as_ref().unwrap().clone(), 0..1);
        RenderCommandResult::Success
    }
}

#[repr(C)]
#[derive(Copy, Clone, Pod, Zeroable)]
struct QuadVertex {
    pub position: [f32; 3],
    pub color: u32,
}

#[repr(C)]
#[derive(Copy, Clone, Pod, Zeroable)]
struct TexturedQuadVertex {
    pub position: [f32; 3],
    pub color: u32,
    pub uv: [f32; 2],
}

#[derive(Component, Eq, PartialEq, Copy, Clone)]
pub struct QuadBatch {
    image_handle_id: HandleId,
    textured: bool,
}

pub struct QuadMeta {
    vertices: BufferVec<QuadVertex>,
    textured_vertices: BufferVec<TexturedQuadVertex>,
    view_bind_group: Option<BindGroup>,
}

impl Default for QuadMeta {
    fn default() -> Self {
        Self {
            vertices: BufferVec::new(BufferUsages::VERTEX),
            textured_vertices: BufferVec::new(BufferUsages::VERTEX),
            view_bind_group: None,
        }
    }
}

#[derive(Default)]
pub struct ImageBindGroups {
    values: HashMap<Handle<Image>, BindGroup>,
}

pub struct QuadPipeline {
    view_layout: BindGroupLayout,
    material_layout: BindGroupLayout,
}

impl FromWorld for QuadPipeline {
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
                    min_binding_size: BufferSize::new(ViewUniform::std140_size_static() as u64),
                },
                count: None,
            }],
            label: Some("quad_view_layout"),
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

        QuadPipeline {
            view_layout,
            material_layout,
        }
    }
}

bitflags::bitflags! {
    #[repr(transparent)]
    // NOTE: Apparently quadro drivers support up to 64x MSAA.
    // MSAA uses the highest 6 bits for the MSAA sample count - 1 to support up to 64x MSAA.
    pub struct QuadPipelineKey: u32 {
        const NONE                        = 0;
        const TEXTURED                    = (1 << 0);
        const MSAA_RESERVED_BITS          = QuadPipelineKey::MSAA_MASK_BITS << QuadPipelineKey::MSAA_SHIFT_BITS;
    }
}

impl QuadPipelineKey {
    const MSAA_MASK_BITS: u32 = 0b111111;
    const MSAA_SHIFT_BITS: u32 = 32 - 6;

    pub fn from_msaa_samples(msaa_samples: u32) -> Self {
        let msaa_bits = ((msaa_samples - 1) & Self::MSAA_MASK_BITS) << Self::MSAA_SHIFT_BITS;
        QuadPipelineKey::from_bits(msaa_bits).unwrap()
    }

    pub fn msaa_samples(&self) -> u32 {
        ((self.bits >> Self::MSAA_SHIFT_BITS) & Self::MSAA_MASK_BITS) + 1
    }
}

impl SpecializedRenderPipeline for QuadPipeline {
    type Key = QuadPipelineKey;

    fn specialize(&self, key: Self::Key) -> RenderPipelineDescriptor {
        let mut formats = vec![
            // position
            VertexFormat::Float32x3,
            // color
            VertexFormat::Unorm8x4,
        ];
        let mut layouts = vec![self.view_layout.clone()];
        let mut shader_defs = Vec::new();

        if key.contains(QuadPipelineKey::TEXTURED) {
            shader_defs.push("TEXTURED".to_string());
            formats.push(VertexFormat::Float32x2); // uv
            layouts.push(self.material_layout.clone());
        }

        let vertex_layout =
            VertexBufferLayout::from_vertex_formats(VertexStepMode::Vertex, formats);

        RenderPipelineDescriptor {
            vertex: VertexState {
                shader: QUAD_SHADER_HANDLE.typed::<Shader>(),
                entry_point: "vertex".into(),
                shader_defs: shader_defs.clone(),
                buffers: vec![vertex_layout],
            },
            fragment: Some(FragmentState {
                shader: QUAD_SHADER_HANDLE.typed::<Shader>(),
                shader_defs,
                entry_point: "fragment".into(),
                targets: vec![ColorTargetState {
                    format: TextureFormat::bevy_default(),
                    blend: Some(BlendState::ALPHA_BLENDING),
                    write_mask: ColorWrites::ALL,
                }],
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
            label: Some("quad_pipeline".into()),
        }
    }
}

const QUAD_INDICES: [usize; 6] = [0, 2, 3, 0, 1, 2];

const QUAD_VERTEX_POSITIONS: [Vec2; 4] = [
    const_vec2!([-0.5, -0.5]),
    const_vec2!([0.5, -0.5]),
    const_vec2!([0.5, 0.5]),
    const_vec2!([-0.5, 0.5]),
];

const QUAD_UVS: [Vec2; 4] = [
    const_vec2!([0., 1.]),
    const_vec2!([1., 1.]),
    const_vec2!([1., 0.]),
    const_vec2!([0., 0.]),
];

#[derive(Component, Clone, Copy)]
pub struct ExtractedQuad {
    pub transform: GlobalTransform,
    pub color: Color,
    /// Select an area of the texture
    pub rect: Option<bevy::sprite::Rect>,
    pub size: Vec2,
    /// Handle to the `Image` of this quad
    /// PERF: storing a `HandleId` instead of `Handle<Image>` enables some optimizations (`ExtractedQuad` becomes `Copy` and doesn't need to be dropped)
    pub image_handle_id: HandleId,
    pub flip_x: bool,
    pub flip_y: bool,
}

#[derive(Default)]
pub struct ExtractedQuads {
    pub quads: Vec<ExtractedQuad>,
}

#[derive(Default)]
pub struct QuadAssetEvents {
    pub images: Vec<AssetEvent<Image>>,
}

pub(crate) fn extract_quad_events(
    mut render_world: ResMut<RenderWorld>,
    mut image_events: EventReader<AssetEvent<Image>>,
) {
    //trace!("extract_quad_events");
    let mut events = render_world.resource_mut::<QuadAssetEvents>();
    let QuadAssetEvents { ref mut images } = *events;
    images.clear();

    for image in image_events.iter() {
        // AssetEvent: !Clone
        images.push(match image {
            AssetEvent::Created { handle } => AssetEvent::Created {
                handle: handle.clone_weak(),
            },
            AssetEvent::Modified { handle } => AssetEvent::Modified {
                handle: handle.clone_weak(),
            },
            AssetEvent::Removed { handle } => AssetEvent::Removed {
                handle: handle.clone_weak(),
            },
        });
    }
}

pub(crate) fn extract_quads(
    mut render_world: ResMut<RenderWorld>,
    texture_atlases: Res<Assets<TextureAtlas>>,
    canvas_query: Query<(Option<&Visibility>, &PietCanvas, &GlobalTransform)>,
    atlas_query: Query<(
        &Visibility,
        &TextureAtlasSprite,
        &GlobalTransform,
        &Handle<TextureAtlas>,
    )>,
) {
    trace!("extract_quads");
    let mut extracted_quads = render_world.resource_mut::<ExtractedQuads>();
    extracted_quads.quads.clear();
    for (opt_visibility, canvas, transform) in canvas_query.iter() {
        if let Some(visibility) = opt_visibility {
            if !visibility.is_visible {
                continue;
            }
        }
        extracted_quads.quads.reserve(canvas.quads().len());
        trace!("canvas: {} quads", canvas.quads().len());
        for quad in canvas.quads() {
            // PERF: we don't check in this function that the `Image` asset is ready, since it should be in most cases and hashing the handle is expensive
            extracted_quads.quads.alloc().init(ExtractedQuad {
                color: quad.color,
                transform: *transform,
                rect: Some(quad.rect),
                size: quad.rect.size(),
                flip_x: quad.flip_x,
                flip_y: quad.flip_y,
                image_handle_id: HandleId::default::<Image>(), // TODO -- handle.id,
            });
        }
    }

    // for (visibility, atlas_quad, transform, texture_atlas_handle) in atlas_query.iter() {
    //     if !visibility.is_visible {
    //         continue;
    //     }
    //     if let Some(texture_atlas) = texture_atlases.get(texture_atlas_handle) {
    //         let rect = Some(texture_atlas.textures[atlas_quad.index as usize]);
    //         extracted_quads.quads.alloc().init(ExtractedQuad {
    //             color: atlas_quad.color,
    //             transform: *transform,
    //             // Select the area in the texture atlas
    //             rect,
    //             // Pass the custom size
    //             size: atlas_quad.custom_size.unwrap_or(Vec2::ONE),
    //             flip_x: atlas_quad.flip_x,
    //             flip_y: atlas_quad.flip_y,
    //             image_handle_id: texture_atlas.texture.id,
    //             anchor: atlas_quad.anchor.as_vec(),
    //         });
    //     }
    // }
}

#[allow(clippy::too_many_arguments)]
pub fn queue_quads(
    mut commands: Commands,
    draw_functions: Res<DrawFunctions<Transparent2d>>,
    render_device: Res<RenderDevice>,
    render_queue: Res<RenderQueue>,
    mut quad_meta: ResMut<QuadMeta>,
    view_uniforms: Res<ViewUniforms>,
    quad_pipeline: Res<QuadPipeline>,
    mut pipelines: ResMut<SpecializedRenderPipelines<QuadPipeline>>,
    mut pipeline_cache: ResMut<PipelineCache>,
    mut image_bind_groups: ResMut<ImageBindGroups>,
    gpu_images: Res<RenderAssets<Image>>,
    msaa: Res<Msaa>,
    mut extracted_quads: ResMut<ExtractedQuads>,
    mut views: Query<&mut RenderPhase<Transparent2d>>,
    events: Res<QuadAssetEvents>,
) {
    trace!("queue_quads");
    // If an image has changed, the GpuImage has (probably) changed
    for event in &events.images {
        match event {
            AssetEvent::Created { .. } => None,
            AssetEvent::Modified { handle } | AssetEvent::Removed { handle } => {
                image_bind_groups.values.remove(handle)
            }
        };
    }

    if let Some(view_binding) = view_uniforms.uniforms.binding() {
        let quad_meta = &mut quad_meta;

        // Clear the vertex buffers
        quad_meta.vertices.clear();
        quad_meta.textured_vertices.clear();

        quad_meta.view_bind_group = Some(render_device.create_bind_group(&BindGroupDescriptor {
            entries: &[BindGroupEntry {
                binding: 0,
                resource: view_binding,
            }],
            label: Some("quad_view_bind_group"),
            layout: &quad_pipeline.view_layout,
        }));

        let draw_quads_function = draw_functions.read().get_id::<DrawPiet>().unwrap();
        let key = QuadPipelineKey::from_msaa_samples(msaa.samples);
        let pipeline = pipelines.specialize(&mut pipeline_cache, &quad_pipeline, key);
        let textured_pipeline = pipelines.specialize(
            &mut pipeline_cache,
            &quad_pipeline,
            key | QuadPipelineKey::TEXTURED,
        );

        // Vertex buffer indices
        let mut index = 0;
        let mut textured_index = 0;

        // FIXME: VisibleEntities is ignored
        for mut transparent_phase in views.iter_mut() {
            let extracted_quads = &mut extracted_quads.quads;
            let image_bind_groups = &mut *image_bind_groups;

            transparent_phase.items.reserve(extracted_quads.len());

            // Sort quads by z for correct transparency and then by handle to improve batching
            extracted_quads.sort_unstable_by(|a, b| {
                match a
                    .transform
                    .translation
                    .z
                    .partial_cmp(&b.transform.translation.z)
                {
                    Some(std::cmp::Ordering::Equal) | None => {
                        a.image_handle_id.cmp(&b.image_handle_id)
                    }
                    Some(other) => other,
                }
            });

            // Impossible starting values that will be replaced on the first iteration
            let mut current_batch = QuadBatch {
                image_handle_id: HandleId::Id(Uuid::nil(), u64::MAX),
                textured: false,
            };
            let mut current_batch_entity = Entity::from_raw(u32::MAX);
            let mut current_image_size = Vec2::ZERO;
            // Add a phase item for each quad, and detect when succesive items can be batched.
            // Spawn an entity with a `QuadBatch` component for each possible batch.
            // Compatible items share the same entity.
            // Batches are merged later (in `batch_phase_system()`), so that they can be interrupted
            // by any other phase item (and they can interrupt other items from batching).
            for extracted_quad in extracted_quads.iter() {
                let new_batch = QuadBatch {
                    image_handle_id: extracted_quad.image_handle_id,
                    textured: extracted_quad.image_handle_id != HandleId::default::<Image>(),
                };
                if new_batch != current_batch {
                    if new_batch.textured {
                        // Set-up a new possible batch
                        if let Some(gpu_image) =
                            gpu_images.get(&Handle::weak(new_batch.image_handle_id))
                        {
                            current_batch = new_batch;
                            current_image_size =
                                Vec2::new(gpu_image.size.width, gpu_image.size.height);
                            current_batch_entity = commands.spawn_bundle((current_batch,)).id();

                            image_bind_groups
                                .values
                                .entry(Handle::weak(current_batch.image_handle_id))
                                .or_insert_with(|| {
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
                                                resource: BindingResource::Sampler(
                                                    &gpu_image.sampler,
                                                ),
                                            },
                                        ],
                                        label: Some("quad_material_bind_group"),
                                        layout: &quad_pipeline.material_layout,
                                    })
                                });
                        } else {
                            // Skip this item if the texture is not ready
                            continue;
                        }
                    } else {
                        current_batch = new_batch;
                        current_batch_entity = commands.spawn_bundle((current_batch,)).id();
                    }
                }

                let mut uvs = QUAD_UVS;
                if current_batch.textured {
                    // Calculate vertex data for this item
                    if extracted_quad.flip_x {
                        uvs = [uvs[1], uvs[0], uvs[3], uvs[2]];
                    }
                    if extracted_quad.flip_y {
                        uvs = [uvs[3], uvs[2], uvs[1], uvs[0]];
                    }

                    // If a rect is specified, adjust UVs and the size of the quad
                    let ratio = 1. / current_image_size;
                    if let Some(rect) = extracted_quad.rect {
                        let rect_size = rect.size();
                        for uv in &mut uvs {
                            *uv = (rect.min + *uv * rect_size) * ratio;
                        }
                    }
                }

                let quad_size = extracted_quad.size;

                // Apply size and global transform
                let positions = QUAD_VERTEX_POSITIONS.map(|quad_pos| {
                    extracted_quad
                        .transform
                        .mul_vec3((quad_pos * quad_size).extend(0.))
                        .into()
                });

                // These items will be sorted by depth with other phase items
                let sort_key = FloatOrd(extracted_quad.transform.translation.z);

                // Store the vertex data and add the item to the render phase
                if current_batch.textured {
                    for i in QUAD_INDICES {
                        quad_meta.textured_vertices.push(TexturedQuadVertex {
                            position: positions[i],
                            color: extracted_quad.color.as_linear_rgba_u32(),
                            uv: uvs[i].into(),
                        });
                    }
                    let item_start = textured_index;
                    textured_index += QUAD_INDICES.len() as u32;
                    let item_end = textured_index;

                    transparent_phase.add(Transparent2d {
                        draw_function: draw_quads_function,
                        pipeline: textured_pipeline,
                        entity: current_batch_entity,
                        sort_key,
                        batch_range: Some(item_start..item_end),
                    });
                } else {
                    for i in QUAD_INDICES {
                        quad_meta.vertices.push(QuadVertex {
                            position: positions[i],
                            color: extracted_quad.color.as_linear_rgba_u32(),
                        });
                    }
                    let item_start = index;
                    index += QUAD_INDICES.len() as u32;
                    let item_end = index;

                    transparent_phase.add(Transparent2d {
                        draw_function: draw_quads_function,
                        pipeline,
                        entity: current_batch_entity,
                        sort_key,
                        batch_range: Some(item_start..item_end),
                    });
                }
            }
        }
        quad_meta
            .vertices
            .write_buffer(&render_device, &render_queue);
        quad_meta
            .textured_vertices
            .write_buffer(&render_device, &render_queue);
        trace!(
            "quads: non-tex={} tex={}",
            quad_meta.vertices.len(),
            quad_meta.textured_vertices.len()
        );
    }
}
