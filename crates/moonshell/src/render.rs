//! Instanced renderer (lifted from the P0 spike, adapted): one draw call per
//! batch of quads — orcs and towers each get a proxy entity + instance buffer.
//! View culling for orcs; towers are always written (few of them).

use bevy::asset::RenderAssetUsages;
use bevy::camera::visibility::NoFrustumCulling;
use bevy::core_pipeline::core_2d::Transparent2d;
use bevy::ecs::query::QueryItem;
use bevy::ecs::system::{
    lifetimeless::{Read, SRes},
    SystemParamItem,
};
use bevy::math::FloatOrd;
use bevy::mesh::{MeshVertexBufferLayoutRef, VertexBufferLayout};
use bevy::prelude::*;
use bevy::render::extract_component::{ExtractComponent, ExtractComponentPlugin};
use bevy::render::mesh::{allocator::MeshAllocator, RenderMesh, RenderMeshBufferInfo};
use bevy::render::render_asset::RenderAssets;
use bevy::render::render_phase::{
    AddRenderCommand, DrawFunctions, PhaseItem, PhaseItemExtraIndex, RenderCommand,
    RenderCommandResult, SetItemPipeline, TrackedRenderPass, ViewSortedRenderPhases,
};
use bevy::render::render_resource::*;
use bevy::render::renderer::{RenderDevice, RenderQueue};
use bevy::render::sync_component::SyncComponent;
use bevy::render::sync_world::MainEntity;
use bevy::render::view::ExtractedView;
use bevy::render::{Render, RenderApp, RenderStartup, RenderSystems};
use bevy::shader::Shader;
use bevy::window::PrimaryWindow;
use bevy::sprite_render::{
    init_mesh_2d_pipeline, Mesh2dPipeline, Mesh2dPipelineKey, RenderMesh2dInstance,
    RenderMesh2dInstances, SetMesh2dBindGroup, SetMesh2dViewBindGroup, ViewKeyCache,
};

use crate::sim::{Orc, Tower};
use crate::ORC_SIZE;

#[repr(C)]
#[derive(Clone, Copy, bytemuck::Pod, bytemuck::Zeroable)]
pub struct InstanceData {
    pub pos_size: [f32; 4],
    pub color: [f32; 4],
}

#[derive(Component, Deref, DerefMut)]
pub struct InstanceMaterialData(pub Vec<InstanceData>);

impl SyncComponent for InstanceMaterialData {
    type Target = Self;
}

impl ExtractComponent for InstanceMaterialData {
    type QueryData = &'static InstanceMaterialData;
    type QueryFilter = ();
    type Out = Self;

    fn extract_component(item: QueryItem<'_, '_, Self::QueryData>) -> Option<Self> {
        Some(InstanceMaterialData(item.0.clone()))
    }
}

#[derive(Component)]
pub struct OrcInstanceBuffer {
    pub buffer: Buffer,
    pub length: usize,
}

#[derive(Resource)]
pub struct InstancedPipeline {
    shader: Handle<Shader>,
    mesh2d_pipeline: Mesh2dPipeline,
}

impl SpecializedMeshPipeline for InstancedPipeline {
    type Key = Mesh2dPipelineKey;

    fn specialize(
        &self,
        key: Self::Key,
        layout: &MeshVertexBufferLayoutRef,
    ) -> Result<RenderPipelineDescriptor, SpecializedMeshPipelineError> {
        let mut descriptor = self.mesh2d_pipeline.specialize(key, layout)?;
        descriptor.vertex.shader = self.shader.clone();
        descriptor.vertex.buffers.push(VertexBufferLayout {
            array_stride: size_of::<InstanceData>() as u64,
            step_mode: VertexStepMode::Instance,
            attributes: vec![
                VertexAttribute {
                    format: VertexFormat::Float32x4,
                    offset: 0,
                    shader_location: 3,
                },
                VertexAttribute {
                    format: VertexFormat::Float32x4,
                    offset: VertexFormat::Float32x4.size(),
                    shader_location: 4,
                },
            ],
        });
        descriptor.fragment.as_mut().unwrap().shader = self.shader.clone();
        Ok(descriptor)
    }
}

fn init_instanced_pipeline(
    mut commands: Commands,
    asset_server: Res<AssetServer>,
    mesh2d_pipeline: Res<Mesh2dPipeline>,
) {
    commands.insert_resource(InstancedPipeline {
        shader: asset_server.add(Shader::from_wgsl(
            include_str!("instanced.wgsl"),
            String::from("instanced.wgsl"),
        )),
        mesh2d_pipeline: mesh2d_pipeline.clone(),
    });
}

fn queue_instanced(
    transparent_2d_draw_functions: Res<DrawFunctions<Transparent2d>>,
    pipeline: Res<InstancedPipeline>,
    mut pipelines: ResMut<SpecializedMeshPipelines<InstancedPipeline>>,
    pipeline_cache: Res<PipelineCache>,
    meshes: Res<RenderAssets<RenderMesh>>,
    render_mesh2d_instances: Res<RenderMesh2dInstances>,
    material_meshes: Query<(Entity, &MainEntity, &InstanceMaterialData)>,
    mut transparent_2d_phases: ResMut<ViewSortedRenderPhases<Transparent2d>>,
    views: Query<(&MainEntity, &ExtractedView)>,
    view_key_cache: Res<ViewKeyCache>,
) {
    let draw = transparent_2d_draw_functions.read().id::<DrawInstancedCmd>();
    for (view_entity, view) in &views {
        let Some(phase) = transparent_2d_phases.get_mut(&view.retained_view_entity) else {
            continue;
        };
        let Some(&view_key) = view_key_cache.get(view_entity) else {
            continue;
        };
        for (entity, main_entity, _) in &material_meshes {
            let Some(RenderMesh2dInstance { mesh_asset_id, .. }) =
                render_mesh2d_instances.get(main_entity)
            else {
                continue;
            };
            let Some(mesh) = meshes.get(*mesh_asset_id) else {
                continue;
            };
            let key = view_key
                | Mesh2dPipelineKey::from_primitive_topology_and_strip_index(
                    mesh.primitive_topology(),
                    mesh.index_format(),
                );
            let pipeline = pipelines
                .specialize(&pipeline_cache, &pipeline, key, &mesh.layout)
                .unwrap();
            phase.add_retained(Transparent2d {
                sort_key: FloatOrd(0.0),
                entity: (entity, *main_entity),
                pipeline,
                draw_function: draw,
                batch_range: 0..1,
                extracted_index: 0,
                extra_index: PhaseItemExtraIndex::None,
                indexed: false,
            });
        }
    }
}

fn prepare_instance_buffers(
    mut commands: Commands,
    mut query: Query<(Entity, &InstanceMaterialData, Option<&mut OrcInstanceBuffer>)>,
    render_device: Res<RenderDevice>,
    render_queue: Res<RenderQueue>,
) {
    for (entity, instance_data, existing) in &mut query {
        let bytes = bytemuck::cast_slice(&instance_data.0);
        let len = instance_data.0.len();
        match existing {
            Some(mut buf) if buf.buffer.size() >= bytes.len() as u64 => {
                render_queue.write_buffer(&buf.buffer, 0, bytes);
                buf.length = len;
            }
            _ => {
                let buffer = render_device.create_buffer_with_data(&BufferInitDescriptor {
                    label: Some("instanced quad buffer"),
                    contents: bytes,
                    usage: BufferUsages::VERTEX | BufferUsages::COPY_DST,
                });
                commands.entity(entity).insert(OrcInstanceBuffer { buffer, length: len });
            }
        }
    }
}

type DrawInstancedCmd = (
    SetItemPipeline,
    SetMesh2dViewBindGroup<0>,
    SetMesh2dBindGroup<1>,
    DrawInstanced,
);

pub struct DrawInstanced;

impl<P: PhaseItem> RenderCommand<P> for DrawInstanced {
    type Param = (
        SRes<RenderAssets<RenderMesh>>,
        SRes<RenderMesh2dInstances>,
        SRes<MeshAllocator>,
    );
    type ViewQuery = ();
    type ItemQuery = Read<OrcInstanceBuffer>;

    fn render<'w>(
        item: &P,
        _view: (),
        instance_buffer: Option<&'w OrcInstanceBuffer>,
        (meshes, render_mesh2d_instances, mesh_allocator): SystemParamItem<'w, '_, Self::Param>,
        pass: &mut TrackedRenderPass<'w>,
    ) -> RenderCommandResult {
        let meshes = meshes.into_inner();
        let render_mesh2d_instances = render_mesh2d_instances.into_inner();
        let mesh_allocator = mesh_allocator.into_inner();
        let Some(RenderMesh2dInstance { mesh_asset_id, .. }) =
            render_mesh2d_instances.get(&item.main_entity())
        else {
            return RenderCommandResult::Skip;
        };
        let Some(gpu_mesh) = meshes.get(*mesh_asset_id) else {
            return RenderCommandResult::Skip;
        };
        let Some(vertex_buffer_slice) = mesh_allocator.mesh_vertex_slice(mesh_asset_id) else {
            return RenderCommandResult::Skip;
        };
        let Some(instance_buffer) = instance_buffer else {
            return RenderCommandResult::Skip;
        };
        pass.set_vertex_buffer(0, vertex_buffer_slice.buffer.slice(..));
        pass.set_vertex_buffer(1, instance_buffer.buffer.slice(..));
        let count = instance_buffer.length as u32;
        match &gpu_mesh.buffer_info {
            RenderMeshBufferInfo::Indexed {
                index_format,
                count: ic,
            } => {
                let Some(index_buffer_slice) = mesh_allocator.mesh_index_slice(mesh_asset_id)
                else {
                    return RenderCommandResult::Skip;
                };
                pass.set_index_buffer(index_buffer_slice.buffer.slice(..), *index_format);
                pass.draw_indexed(
                    index_buffer_slice.range.start..(index_buffer_slice.range.start + ic),
                    vertex_buffer_slice.range.start as i32,
                    0..count,
                );
            }
            RenderMeshBufferInfo::NonIndexed => {
                pass.draw(vertex_buffer_slice.range, 0..count);
            }
        }
        RenderCommandResult::Success
    }
}

pub struct InstancedRenderPlugin;

impl Plugin for InstancedRenderPlugin {
    fn build(&self, app: &mut App) {
        app.add_plugins(ExtractComponentPlugin::<InstanceMaterialData>::default())
            .add_systems(Update, (write_orc_instances, write_tower_instances));

        let Some(render_app) = app.get_sub_app_mut(RenderApp) else {
            return;
        };
        render_app
            .add_render_command::<Transparent2d, DrawInstancedCmd>()
            .init_resource::<SpecializedMeshPipelines<InstancedPipeline>>()
            .add_systems(RenderStartup, init_instanced_pipeline.after(init_mesh_2d_pipeline))
            .add_systems(
                Render,
                (
                    queue_instanced.in_set(RenderSystems::QueueMeshes),
                    prepare_instance_buffers.in_set(RenderSystems::PrepareResources),
                ),
            );
    }
}

/// View rect for culling (world units) from camera + window.
fn view_cull_rect(
    camera: &mut Query<(&GlobalTransform, &Projection), With<Camera>>,
    window: Option<&Window>,
) -> Option<Rect> {
    let (ct, proj) = camera.single().ok()?;
    let win = window?;
    let scale = match proj {
        Projection::Orthographic(p) => p.scale.max(1e-6),
        _ => return None,
    };
    let half = Vec2::new(win.width() / (2.0 * scale), win.height() / (2.0 * scale))
        + Vec2::splat(ORC_SIZE);
    Some(Rect::from_center_size(ct.translation().truncate(), half * 2.0))
}

fn write_orc_instances(
    orcs: Query<(&Transform, &Orc)>,
    mut data: Query<&mut InstanceMaterialData, With<OrcProxy>>,
    mut camera: Query<(&GlobalTransform, &Projection), With<Camera>>,
    window: Query<&Window, With<PrimaryWindow>>,
) {
    let Ok(mut data) = data.single_mut() else {
        return;
    };
    let view = view_cull_rect(&mut camera, window.single().ok());
    data.0.clear();
    for (tf, orc) in &orcs {
        let p = tf.translation.truncate();
        if let Some(r) = view {
            if !r.contains(p) {
                continue;
            }
        }
        data.0.push(InstanceData {
            pos_size: [p.x, p.y, ORC_SIZE, ORC_SIZE],
            color: orc_color(orc.seed),
        });
    }
}

fn write_tower_instances(
    towers: Query<(&Transform, &Tower)>,
    mut data: Query<&mut InstanceMaterialData, With<TowerProxy>>,
) {
    let Ok(mut data) = data.single_mut() else {
        return;
    };
    data.0.clear();
    for (tf, _tower) in &towers {
        let p = tf.translation.truncate();
        data.0.push(InstanceData {
            pos_size: [p.x, p.y, ORC_SIZE * 3.0, ORC_SIZE * 3.0],
            color: [0.2, 0.7, 0.95, 1.0], // towers: cyan
        });
    }
}

fn orc_color(seed: u32) -> [f32; 4] {
    let g = 0.55 + 0.25 * ((seed % 5) as f32 / 5.0);
    [0.12, g, 0.22, 1.0]
}

/// Marker so the two proxies (orcs vs towers) are distinct targets.
#[derive(Component)]
pub struct OrcProxy;
#[derive(Component)]
pub struct TowerProxy;

pub fn quad_mesh() -> Mesh {
    let positions: Vec<[f32; 3]> = vec![
        [-0.5, -0.5, 0.0],
        [0.5, -0.5, 0.0],
        [0.5, 0.5, 0.0],
        [-0.5, -0.5, 0.0],
        [0.5, 0.5, 0.0],
        [-0.5, 0.5, 0.0],
    ];
    Mesh::new(PrimitiveTopology::TriangleList, RenderAssetUsages::RENDER_WORLD)
        .with_inserted_attribute(Mesh::ATTRIBUTE_POSITION, positions)
}

pub fn spawn_proxies(mut commands: Commands, mut meshes: ResMut<Assets<Mesh>>) {
    let mesh = meshes.add(quad_mesh());
    commands.spawn((
        Mesh2d(mesh.clone()),
        InstanceMaterialData(Vec::with_capacity(200_000)),
        Transform::default(),
        Visibility::default(),
        NoFrustumCulling,
        OrcProxy,
    ));
    commands.spawn((
        Mesh2d(mesh),
        InstanceMaterialData(Vec::with_capacity(1024)),
        Transform::default(),
        Visibility::default(),
        NoFrustumCulling,
        TowerProxy,
    ));
}
