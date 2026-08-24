// Instanced quad renderer (P0 spike shader): position+size @3, color @4.
#import bevy_sprite::mesh2d_view_bindings::view

#ifdef SRGB_OUTPUT
#import bevy_render::color_operations::linear_to_srgb
#endif

struct VertexOutput {
    @builtin(position) clip_position: vec4<f32>,
    @location(0) color: vec4<f32>,
};

@vertex
fn vertex(
    @location(0) position: vec3<f32>,
    @location(3) pos_size: vec4<f32>,
    @location(4) color: vec4<f32>,
) -> VertexOutput {
    var out: VertexOutput;
    let world = vec4<f32>(position.xy + pos_size.xy, position.z, 1.0);
    out.clip_position = view.clip_from_world * world;
    out.color = color;
    return out;
}

@fragment
fn fragment(in: VertexOutput) -> @location(0) vec4<f32> {
#ifdef SRGB_OUTPUT
    return vec4<f32>(linear_to_srgb(in.color.rgb), in.color.a);
#else
    return in.color;
#endif
}
