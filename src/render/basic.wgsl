struct Camera {
    view_projection: mat4x4<f32>,
};

@group(0) @binding(0)
var<uniform> camera: Camera;

struct Model {
    matrix: mat4x4<f32>,
};

@group(0) @binding(1)
var<uniform> model: Model;

struct VertexInput {
    @location(0) position: vec3<f32>,
    @location(1) color: vec3<f32>,
};

struct VertexOutput {
    @builtin(position) position: vec4<f32>,
    @location(0) color: vec3<f32>,
    @location(1) world_position: vec3<f32>,
};

@vertex
fn vs_main(input: VertexInput) -> VertexOutput {
    var output: VertexOutput;
    let world = model.matrix * vec4<f32>(input.position, 1.0);
    output.position = camera.view_projection * world;
    output.world_position = world.xyz;
    output.color = input.color;
    return output;
}

@fragment
fn fs_main(input: VertexOutput) -> @location(0) vec4<f32> {
    return vec4<f32>(input.color, 1.0);
}

@fragment
fn fs_pixel(input: VertexOutput) -> @location(0) vec4<f32> {
    // Y de pantalla crece hacia abajo. El culling deja las caras exteriores.
    let area = cross(dpdy(input.world_position), dpdx(input.world_position));
    let normal = area * inverseSqrt(max(dot(area, area), 1e-30));
    let light = normalize(vec3(-0.45, 0.85, 0.4));
    let diffuse = dot(normal, light);
    let shade = select(select(0.58, 0.78, diffuse >= 0.15), 1.0, diffuse >= 0.60);
    return vec4(input.color * shade, 1.0);
}
