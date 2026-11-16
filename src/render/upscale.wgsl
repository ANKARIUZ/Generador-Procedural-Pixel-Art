@group(0) @binding(0) var source: texture_2d<f32>;
@group(0) @binding(1) var nearest_sampler: sampler;

struct VertexOutput {
    @builtin(position) position: vec4<f32>,
    @location(0) uv: vec2<f32>,
};

@vertex
fn vs_main(@builtin(vertex_index) index: u32) -> VertexOutput {
    let uv = array<vec2<f32>, 3>(vec2(0.0, 0.0), vec2(2.0, 0.0), vec2(0.0, 2.0))[index];
    return VertexOutput(vec4(uv * vec2(2.0, -2.0) + vec2(-1.0, 1.0), 0.0, 1.0), uv);
}

@fragment
fn fs_main(input: VertexOutput) -> @location(0) vec4<f32> {
    return textureSample(source, nearest_sampler, input.uv);
}
