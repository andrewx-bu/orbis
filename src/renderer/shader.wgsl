struct CameraUniform {
    view_projection: mat4x4f,
}

struct DirectionalLightUniform {
    direction_to_light: vec3f,
    ambient_strength: f32,
}

struct VertexOutput {
    @builtin(position) position: vec4f,
    @location(0) elevation: f32,
    @location(1) normal: vec3f,
    @location(2) @interpolate(flat) debug_color: vec3f,
    @location(3) patch_uv: vec2f,
    @location(4) surface_direction: vec3f,
}

struct VertexInput {
    @location(0) position: vec3f,
    @location(1) elevation: f32,
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
    output.elevation = input.elevation;
    output.surface_direction = normalize(input.position);
    output.normal = input.normal;
    output.debug_color = input.debug_color;
    output.patch_uv = input.patch_uv;
    return output;
}

// Integer hashing keeps the material pattern anchored to the planet, across cube faces.
fn lattice_value(cell: vec3i) -> f32 {
    let c = bitcast<vec3u>(cell);
    var h = c.x * 1597334677u ^ c.y * 3812015801u ^ c.z * 2798796415u;
    h = (h ^ (h >> 16u)) * 2246822519u;
    h = h ^ (h >> 13u);
    return f32(h >> 8u) / 16777215.0;
}

fn material_noise(point: vec3f) -> f32 {
    let cell = vec3i(floor(point));
    let f = fract(point);
    let t = f * f * (3.0 - 2.0 * f);
    let a = mix(lattice_value(cell), lattice_value(cell + vec3i(1, 0, 0)), t.x);
    let b = mix(lattice_value(cell + vec3i(0, 1, 0)), lattice_value(cell + vec3i(1, 1, 0)), t.x);
    let c = mix(lattice_value(cell + vec3i(0, 0, 1)), lattice_value(cell + vec3i(1, 0, 1)), t.x);
    let d = mix(lattice_value(cell + vec3i(0, 1, 1)), lattice_value(cell + vec3i(1, 1, 1)), t.x);
    return mix(mix(a, b, t.y), mix(c, d, t.y), t.z) * 2.0 - 1.0;
}

fn terrain_color(input: VertexOutput) -> vec3f {
    let up = normalize(input.surface_direction);
    let slope = 1.0 - clamp(dot(normalize(input.normal), up), 0.0, 1.0);
    // Fade each noise band when its cells become smaller than a few pixels.
    let footprint = max(length(dpdx(up)), length(dpdy(up)));
    let broad = material_noise(up * 48.0) * (1.0 - smoothstep(0.25, 0.75, footprint * 48.0));
    let fine = material_noise(up * 192.0) * (1.0 - smoothstep(0.25, 0.75, footprint * 192.0));
    let elevation = input.elevation + broad * 0.045 + fine * 0.015;
    let vegetation = mix(vec3f(0.07, 0.19, 0.06), vec3f(0.24, 0.29, 0.12),
        smoothstep(-0.15, 0.25, elevation));
    let rock_weight = max(smoothstep(0.03, 0.18, slope), smoothstep(0.20, 0.45, elevation));
    let snow_weight = smoothstep(0.38, 0.52, elevation) * (1.0 - smoothstep(0.05, 0.20, slope));
    let ground = mix(vegetation, vec3f(0.30, 0.27, 0.23), rock_weight);
    let material = mix(ground, vec3f(0.85, 0.90, 0.94), snow_weight);
    return material * (1.0 + broad * 0.08 + fine * 0.04);
}

@fragment
fn fragment_main(input: VertexOutput) -> @location(0) vec4f {
    let diffuse_strength = max(dot(normalize(input.normal), light.direction_to_light), 0.0);
    let light_strength = light.ambient_strength
        + (1.0 - light.ambient_strength) * diffuse_strength;
    return vec4f(terrain_color(input) * light_strength, 1.0);
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
