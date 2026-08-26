struct CameraUniform {
    view_projection: mat4x4f,
}

struct DirectionalLightUniform {
    direction_to_light: vec3f,
    ambient_strength: f32,
}

struct VertexOutput {
    @builtin(position) position: vec4f,
    @location(0) color: vec3f,
    @location(1) normal: vec3f,
}

struct VertexInput {
    @location(0) position: vec3f,
    @location(1) color: vec3f,
    @location(2) normal: vec3f,
}

@group(0) @binding(0)
var<uniform> camera: CameraUniform;

@group(1) @binding(0)
var<uniform> light: DirectionalLightUniform;

@vertex
fn vertex_main(input: VertexInput) -> VertexOutput {
    var output: VertexOutput;
    output.position = camera.view_projection * vec4f(input.position, 1.0);
    output.color = input.color;
    output.normal = input.normal;
    return output;
}

@fragment
fn fragment_main(input: VertexOutput) -> @location(0) vec4f {
    let diffuse_strength = max(dot(normalize(input.normal), light.direction_to_light), 0.0);
    let light_strength = light.ambient_strength
        + (1.0 - light.ambient_strength) * diffuse_strength;
    return vec4f(input.color * light_strength, 1.0);
}
