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
    @location(2) @interpolate(flat) debug_color: vec3f,
    @location(3) patch_uv: vec2f,
}

struct VertexInput {
    @location(0) position: vec3f,
    @location(1) color: vec3f,
    @location(2) normal: vec3f,
    @location(3) debug_color: vec3f,
    @location(4) patch_uv: vec2f,
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
    output.debug_color = input.debug_color;
    output.patch_uv = input.patch_uv;
    return output;
}

@fragment
fn fragment_main(input: VertexOutput) -> @location(0) vec4f {
    let diffuse_strength = max(dot(normalize(input.normal), light.direction_to_light), 0.0);
    let light_strength = light.ambient_strength
        + (1.0 - light.ambient_strength) * diffuse_strength;
    return vec4f(input.color * light_strength, 1.0);
}

@fragment
fn fragment_patch_debug(input: VertexOutput) -> @location(0) vec4f {
    // Scale distance to the patch boundary into pixels for consistent line width.
    let edge_distance = min(input.patch_uv, vec2f(1.0) - input.patch_uv);
    let pixel_distance = edge_distance / max(fwidth(input.patch_uv), vec2f(1.0e-6));
    let interior = smoothstep(0.5, 1.5, min(pixel_distance.x, pixel_distance.y));
    let color = mix(vec3f(0.02), input.debug_color, interior);
    return vec4f(color, 1.0);
}
