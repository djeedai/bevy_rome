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
    math::{const_vec2, Mat2},
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
            BlendState, Buffer, BufferBinding, BufferBindingType, BufferInitDescriptor, BufferSize,
            BufferUsages, BufferVec, ColorTargetState, ColorWrites, FragmentState, FrontFace,
            IndexFormat, MultisampleState, PipelineCache, PolygonMode, PrimitiveState,
            PrimitiveTopology, RenderPipelineDescriptor, SamplerBindingType, ShaderStages,
            SpecializedRenderPipeline, SpecializedRenderPipelines, TextureFormat,
            TextureSampleType, TextureViewDimension, VertexBufferLayout, VertexFormat, VertexState,
            VertexStepMode,
        },
        renderer::{RenderDevice, RenderQueue},
        texture::{BevyDefault, Image},
        view::{Msaa, ViewUniform, ViewUniformOffset, ViewUniforms},
        RenderWorld,
    },
    sprite::Rect as SRect,
    utils::HashMap,
};
use copyless::VecHelper;

use crate::{Canvas, PRIMITIVE_SHADER_HANDLE};

pub type DrawPrimitive = (
    SetItemPipeline,
    SetPrimitiveViewBindGroup<0>,
    SetPrimitiveBufferBindGroup<1>,
    //SetQuadTextureBindGroup<2>,
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

// pub struct SetQuadTextureBindGroup<const I: usize>;

// impl<const I: usize> EntityRenderCommand for SetQuadTextureBindGroup<I> {
//     type Param = (SRes<ImageBindGroups>, SQuery<Read<PrimitiveBatch>>);

//     fn render<'w>(
//         _view: Entity,
//         item: Entity,
//         (image_bind_groups, query_batch): SystemParamItem<'w, '_, Self::Param>,
//         pass: &mut TrackedRenderPass<'w>,
//     ) -> RenderCommandResult {
//         let primitive_batch = query_batch.get(item).unwrap();
//         if primitive_batch.textured {
//             let image_bind_groups = image_bind_groups.into_inner();
//             pass.set_bind_group(
//                 I,
//                 image_bind_groups
//                     .values
//                     .get(&Handle::weak(primitive_batch.image_handle_id))
//                     .unwrap(),
//                 &[],
//             );
//         }
//         RenderCommandResult::Success
//     }
// }

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
        // if primitive_batch.textured {
        //     pass.set_vertex_buffer(
        //         0,
        //         primitive_meta.textured_vertices.buffer().unwrap().slice(..),
        //     );
        // } else {
        //     pass.set_vertex_buffer(0, primitive_meta.vertices.buffer().unwrap().slice(..));
        // }
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
    //image_handle_id: HandleId,
    //textured: bool,
    index_buffer: Buffer,
    canvas_entity: Entity,
}

pub struct CanvasMeta {
    /// Entity the `Canvas` component is attached to.
    canvas_entity: Entity,
    /// Entity the `PrimitiveBatch` component is attached to (render phase item).
    batch_entity: Entity,
    primitive_bind_group: BindGroup,
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
                    min_binding_size: BufferSize::new(ViewUniform::std140_size_static() as u64),
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
        // let mut formats = vec![
        //     // position
        //     VertexFormat::Float32x3,
        //     // color
        //     VertexFormat::Unorm8x4,
        // ];
        let mut layouts = vec![self.view_layout.clone(), self.prim_layout.clone()];
        let mut shader_defs = Vec::new();

        if key.contains(PrimitivePipelineKey::TEXTURED) {
            shader_defs.push("TEXTURED".to_string());
            //formats.push(VertexFormat::Float32x2); // uv
            layouts.push(self.material_layout.clone());
        }

        // let vertex_layout =
        //     VertexBufferLayout::from_vertex_formats(VertexStepMode::Vertex, formats);

        RenderPipelineDescriptor {
            vertex: VertexState {
                shader: PRIMITIVE_SHADER_HANDLE.typed::<Shader>(),
                entry_point: "vertex".into(),
                shader_defs: shader_defs.clone(),
                buffers: vec![], //vec![vertex_layout],
            },
            fragment: Some(FragmentState {
                shader: PRIMITIVE_SHADER_HANDLE.typed::<Shader>(),
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
            label: Some("primitive_pipeline".into()),
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

#[derive(Default)]
pub struct ExtractedCanvas {
    pub transform: GlobalTransform,
    pub buffer: Vec<f32>,
    pub indices: Vec<u32>,
    storage: Option<Buffer>,
    storage_capacity: usize,
    index_buffer: Option<Buffer>,
    index_buffer_capacity: usize,
}

impl ExtractedCanvas {
    /// Write the CPU scratch buffer into the associated GPU storage buffer.
    pub fn write_buffer(&mut self, render_device: &RenderDevice, render_queue: &RenderQueue) {
        // Primitive buffer
        let size = self.buffer.len();
        let contents = bytemuck::cast_slice(&self.buffer[..]);
        if size > self.storage_capacity {
            // GPU buffer too small; reallocated...
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
        let size = self.indices.len();
        let contents = bytemuck::cast_slice(&self.indices[..]);
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

#[derive(Default)]
pub struct ExtractedCanvases {
    /// Map from app world's entity with a canvas to associated render world's extracted canvas.
    pub canvases: HashMap<Entity, ExtractedCanvas>,
}

#[derive(Default)]
pub struct PrimitiveAssetEvents {
    pub images: Vec<AssetEvent<Image>>,
}

pub(crate) fn extract_primitive_events(
    mut render_world: ResMut<RenderWorld>,
    mut image_events: EventReader<AssetEvent<Image>>,
) {
    //trace!("extract_primitive_events");
    let mut events = render_world.resource_mut::<PrimitiveAssetEvents>();
    let PrimitiveAssetEvents { ref mut images } = *events;
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

pub(crate) fn extract_primitives(
    mut render_world: ResMut<RenderWorld>,
    _texture_atlases: Res<Assets<TextureAtlas>>,
    mut canvas_query: Query<(Entity, Option<&Visibility>, &mut Canvas, &GlobalTransform)>,
    _atlas_query: Query<(
        &Visibility,
        &TextureAtlasSprite,
        &GlobalTransform,
        &Handle<TextureAtlas>,
    )>,
) {
    trace!("extract_primitives");
    let mut extracted_canvases = render_world.resource_mut::<ExtractedCanvases>();
    extracted_canvases.canvases.clear();
    for (entity, opt_visibility, mut canvas, transform) in canvas_query.iter_mut() {
        if let Some(visibility) = opt_visibility {
            if !visibility.is_visible {
                continue;
            }
        }
        let prim_buffer = canvas.take_primitives_buffer();
        let idx_buffer = canvas.take_indices_buffer();
        if !prim_buffer.is_empty() && !idx_buffer.is_empty() {
            trace!(
                "canvas: primitives = {} f32, indices = {} u32",
                prim_buffer.len(),
                idx_buffer.len()
            );
            for f in &prim_buffer {
                trace!("f32: {}", f);
            }
            for u in &idx_buffer {
                trace!("u32: {:x}", u);
            }
            let extracted_canvas =
                extracted_canvases
                    .canvases
                    .entry(entity)
                    .or_insert(ExtractedCanvas {
                        transform: *transform,
                        ..Default::default()
                    });
            extracted_canvas.buffer = prim_buffer;
            extracted_canvas.indices = idx_buffer;
        }
    }

    // for (visibility, atlas_quad, transform, texture_atlas_handle) in atlas_query.iter() {
    //     if !visibility.is_visible {
    //         continue;
    //     }
    //     if let Some(texture_atlas) = texture_atlases.get(texture_atlas_handle) {
    //         let rect = Some(texture_atlas.textures[atlas_quad.index as usize]);
    //         extracted_canvases.quads.alloc().init(ExtractedQuad {
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
pub fn queue_primitives(
    mut commands: Commands,
    draw_functions: Res<DrawFunctions<Transparent2d>>,
    render_device: Res<RenderDevice>,
    render_queue: Res<RenderQueue>,
    mut primitive_meta: ResMut<PrimitiveMeta>,
    view_uniforms: Res<ViewUniforms>,
    primitive_pipeline: Res<PrimitivePipeline>,
    mut pipelines: ResMut<SpecializedRenderPipelines<PrimitivePipeline>>,
    mut pipeline_cache: ResMut<PipelineCache>,
    mut image_bind_groups: ResMut<ImageBindGroups>,
    _gpu_images: Res<RenderAssets<Image>>,
    msaa: Res<Msaa>,
    mut extracted_canvases: ResMut<ExtractedCanvases>,
    mut views: Query<&mut RenderPhase<Transparent2d>>,
    events: Res<PrimitiveAssetEvents>,
) {
    trace!("queue_primitives");

    // If an Image has changed, the GpuImage has (probably) changed
    for event in &events.images {
        match event {
            AssetEvent::Created { .. } => None,
            AssetEvent::Modified { handle } | AssetEvent::Removed { handle } => {
                image_bind_groups.values.remove(handle)
            }
        };
    }

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
    let pipeline = pipelines.specialize(&mut pipeline_cache, &primitive_pipeline, key);
    // let textured_pipeline = pipelines.specialize(
    //     &mut pipeline_cache,
    //     &primitive_pipeline,
    //     key | PrimitivePipelineKey::TEXTURED,
    // );

    for (canvas_entity, extracted_canvas) in extracted_canvases.canvases.iter_mut() {
        // FIXME - move to PREPARE phase?
        extracted_canvas.write_buffer(&render_device, &render_queue);

        let canvas_meta = primitive_meta
            .canvas_meta
            .entry(*canvas_entity)
            .or_insert_with(|| {
                let primitive_bind_group = render_device.create_bind_group(&BindGroupDescriptor {
                    entries: &[BindGroupEntry {
                        binding: 0,
                        resource: extracted_canvas.binding().unwrap(),
                    }],
                    label: Some("prim_bind_group"),
                    layout: &primitive_pipeline.prim_layout,
                });

                let index_buffer = extracted_canvas.index_buffer.as_ref().unwrap().clone();

                trace!("Adding new CanvasMeta: canvas_entity={:?}", canvas_entity);

                CanvasMeta {
                    canvas_entity: *canvas_entity,
                    batch_entity: Entity::from_raw(0), // fixed below
                    primitive_bind_group,
                    index_buffer,
                }
            });

        // Render world entities are deleted each frame, re-add
        let primitive_batch = PrimitiveBatch {
            index_buffer: canvas_meta.index_buffer.clone(),
            canvas_entity: *canvas_entity,
        };
        let batch_entity = commands.spawn_bundle((primitive_batch,)).id();
        canvas_meta.batch_entity = batch_entity;

        trace!(
            "CanvasMeta: canvas_entity={:?} batch_entity={:?}",
            canvas_entity,
            batch_entity
        );

        let sort_key = FloatOrd(extracted_canvas.transform.translation.z);
        let index_count = extracted_canvas.indices.len() as u32;

        if index_count > 0 {
            if let Some(index_buffer) = &extracted_canvas.index_buffer {
                for mut transparent_phase in views.iter_mut() {
                    trace!(
                        "Add Transparent2d item: 0..{} (sort={:?})",
                        index_count,
                        sort_key
                    );
                    transparent_phase.add(Transparent2d {
                        draw_function: draw_primitives_function,
                        pipeline,
                        entity: canvas_meta.batch_entity,
                        sort_key,
                        batch_range: Some(0..index_count),
                    });
                }
            }
        }
    }
}
