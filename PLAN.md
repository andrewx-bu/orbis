# Orbis Project Plan

Orbis is an open-source procedural planet renderer and simulation project written in Rust.
This is a flexible learning project, so its scope, architecture, and milestones may change as the work develops.

The project is intended to build practical understanding of:

- Rust systems programming
- `wgpu`, WGSL, and GPU programming
- real-time rendering and graphics pipelines
- procedural generation
- level-of-detail and large-world rendering
- parallel computation and GPU compute
- simulation, profiling, and performance optimization

## 1. Project Goal

Orbis will generate deterministic procedural planets that users can explore continuously from orbit to ground level.
The engine will generate, cache, render, and evict world data around the camera instead of storing one full-resolution planetary mesh.

The target experience is:

1. Generate or load a planet from a seed and configuration.
2. View and orbit the whole planet.
3. Descend through continental, regional, and local terrain scales.
4. Explore detailed terrain near the surface.
5. Return to orbit without a loading-screen transition.

Given the same generation version, seed, coordinates, and parameters, the engine should reproduce the same world data.
This allows terrain to be discarded and regenerated as needed.

## 2. Scope

The first priority is a working procedural planet renderer.
Simulation and content systems will build on the renderer only after its foundations are stable.

Orbis is not a general-purpose game engine.
Audio, multiplayer, generic scripting, unrelated physics, and other general game-engine systems are outside the planned scope.

### Primary Scope

- Native desktop rendering
- Deterministic procedural planets
- Cube-sphere terrain
- Quadtree level of detail
- Seamless orbital-to-ground navigation
- Large-world coordinate precision
- Terrain streaming and caching
- GPU terrain generation
- Oceans and atmospheric rendering
- Debug views and performance instrumentation

### Later Extensions

- Improved terrain generation and erosion
- Climate and biome generation
- Rocks, trees, shrubs, and grass
- Clouds, weather, seasons, and accelerated time
- Planet saving and sharing
- Regional mesh and heightmap export
- WebAssembly and WebGPU demo
- Real Earth data

These extensions remain part of the project direction, but none should block completion of the renderer.

## 3. Technical Foundation

The initial target is a native desktop application built with:

- Rust
- `wgpu`
- WGSL
- `winit`
- Cargo

The core renderer will not use a full game engine so its rendering and resource systems remain directly accessible.
Additional libraries should be introduced only for clear, specific needs.
A browser build may be added after the native renderer is mature.

The repository should begin with a simple structure and split into crates or subsystems only when concrete boundaries emerge.

## 4. Core Systems

### Rendering Foundation

The first renderer will establish:

- GPU device, queue, and surface management
- render pipelines and shaders
- vertex, index, uniform, and storage buffers
- textures and depth buffers
- model, view, and projection transforms
- camera controls
- depth testing
- basic lighting

The renderer should stay small and observable while these foundations are developed.

### Planet Geometry

The planned planet representation is a cube sphere.
Each of the six cube faces will contain a quadtree of terrain patches projected onto a sphere.
This provides a practical structure for independent subdivision, generation, culling, and streaming.

The representation should be validated with a prototype before it becomes a fixed architectural choice.

### Level of Detail

Terrain detail should be selected using camera distance and projected geometric error.
The system will need to address:

- quadtree traversal
- frustum and horizon culling
- neighboring patch constraints
- cracks between LOD levels
- popping and transition stability
- patch caching and memory budgets

Skirts are a reasonable first solution for cracks.
More advanced stitching or geomorphing can be evaluated later.

### Procedural Terrain

Terrain generation will begin with deterministic multi-scale noise.
It can later expand to include:

- continental masks
- mountain distributions
- domain warping
- oceans and coastlines
- erosion
- rivers and drainage
- climate and biome inputs

The same inputs must always produce the same terrain within a generation version.

### Terrain Streaming

The engine will request terrain based on camera position and visible LOD.
Patches will move through a lifecycle such as requested, generating, resident, cached, and evicted.

The streaming system may include:

- prioritized generation queues
- asynchronous jobs
- cancellation of obsolete work
- resource pools
- bounded caches
- prefetching
- explicit CPU and GPU ownership

The initial implementation should remain synchronous until measurement shows that asynchronous scheduling is needed.

### Large-World Coordinates

Planetary rendering requires stable coordinates across very different scales.
The likely approach is:

- `f64` world positions on the CPU
- camera-relative local coordinates
- `f32` positions on the GPU
- local tangent frames where useful

The final approach should be selected after reproducing and measuring precision problems.

### GPU Compute

GPU compute should eventually handle workloads that benefit from parallel execution, including:

- height generation
- normal generation
- erosion
- vegetation candidate generation
- visibility and culling
- indirect draw preparation
- weather simulation

CPU and GPU implementations should be compared before moving work permanently to compute shaders.
Synchronization, memory bandwidth, and dispatch overhead should be measured as part of those comparisons.

### Surface Rendering

Visual development should progress from simple lighting to:

- terrain materials
- directional sunlight
- oceans
- atmospheric scattering
- shadows
- fog and aerial perspective
- sky rendering

Atmosphere and oceans are high-value features for the orbital view, but they should follow stable terrain rendering.

### Ground Detail

Near-surface detail may include rocks, trees, shrubs, and grass.
Placement should be deterministic and filtered by terrain, slope, climate, and biome data.
GPU instancing and distance-based LOD should be used to keep object counts manageable.

## 5. Simulation and Content Extensions

### Climate and Biomes

Climate generation may derive temperature and moisture from latitude, elevation, water proximity, terrain barriers, and prevailing winds.
These fields can drive biome classification, terrain materials, vegetation, and snow.

### Weather and Clouds

Weather should progress through independent levels:

1. Visual states such as fog, rain, snow, wind, and storms.
2. Location-aware weather driven by climate and terrain.
3. A coarse global atmospheric simulation using GPU compute.

Cloud rendering can begin with a global procedural layer and later progress toward volumetric clouds driven by simulated humidity.

### Time and Seasons

Later simulation work may include:

- day and night cycles
- planetary rotation
- axial tilt and seasons
- accelerated simulation time
- snow accumulation
- changing climate and vegetation

These are research directions rather than requirements for the core renderer.

## 6. Saving, Sharing, and Export

A saved planet should primarily contain its procedural definition rather than generated geometry.
The definition may include:

- generation version
- seed
- radius and ocean level
- terrain parameters
- climate parameters
- atmosphere parameters
- vegetation parameters

Versioning is required because changes to generation algorithms may change a planet produced from the same seed.

Possible export features include:

- planet configuration files
- selected terrain regions as glTF or GLB
- heightmaps
- normal maps
- biome and material masks

High-resolution geometry export should be regional rather than global.

## 7. Web and Earth Support

A future WebAssembly and WebGPU build could provide a reduced interactive demo in the browser.
Native desktop should remain the primary platform for development, debugging, and profiling.

Real Earth support is a stretch feature that may reuse the same renderer with alternative data sources.
Possible inputs include elevation tiles, geographic vectors, roads, and building footprints.
Earth support should begin only after procedural planet rendering is mature.

## 8. Observability and Performance

Debug views are part of the renderer, not optional development leftovers.
Useful views include:

- wireframe and normals
- depth and overdraw
- chunk boundaries
- quadtree and LOD levels
- biome and climate fields
- wind and precipitation

The application should eventually report:

- frame time and frame rate
- CPU and GPU timings
- draw calls and triangle counts
- visible and resident patches
- cache and memory usage
- generation queue state
- compute dispatch timings

Performance decisions should be based on repeatable measurements.
Useful comparisons include CPU versus GPU generation, LOD thresholds, patch sizes, culling strategies, and cache budgets.

## 9. Development Roadmap

The phases are ordered dependencies, not deadlines.
Each phase should end with a runnable and visually demonstrable result.

### Phase 1: Rendering Fundamentals

- Create the Cargo project and basic checks.
- Open a window and initialize `wgpu`.
- Render a triangle and indexed cube.
- Add a movable camera, depth testing, and basic lighting.

Deliverable: a small native 3D renderer.

### Phase 2: Procedural Terrain

- Render a flat terrain grid.
- Add deterministic height generation and normals.
- Divide terrain into independently generated patches.
- Add patch caching and debug visualization.

Deliverable: a navigable, seed-based terrain viewer larger than one mesh.

### Phase 3: Terrain LOD

- Implement quadtree subdivision.
- Select LOD using screen-space error.
- Add frustum culling.
- Resolve cracks and reduce popping.
- Add LOD and chunk debug views.

Deliverable: scalable terrain across large viewing distances.

### Phase 4: Procedural Planet

- Project terrain patches onto six cube-sphere faces.
- Adapt LOD and culling to a sphere.
- Add orbital navigation.
- Implement camera-relative rendering.
- Support continuous travel from orbit toward the surface.

Deliverable: a stable, explorable procedural planet.

This is the minimum successful version of Orbis.

### Phase 5: GPU Generation and Visual Quality

- Move selected terrain work to compute shaders.
- Profile CPU and GPU generation paths.
- Improve continents, mountains, oceans, and terrain materials.
- Add atmosphere, sky rendering, and aerial perspective.
- Add performance instrumentation.

Deliverable: a visually compelling planet with meaningful GPU-driven generation.

### Phase 6: Ground Detail and World Data

- Add climate and biome fields.
- Add deterministic rocks and vegetation.
- Implement instancing and vegetation LOD.
- Add planet save files and regional export.

Deliverable: a detailed surface that can be saved, shared, and partially exported.

### Phase 7: Optional Advanced Work

Select features based on interest and the stability of earlier systems:

- erosion and river networks
- clouds and weather
- atmospheric simulation
- seasons and accelerated time
- browser demo
- real Earth data
- GPU-driven culling and indirect rendering

Deliverable: one or more focused research extensions built on the mature renderer.

## 10. Feasibility and Risk

The core renderer is achievable as an incremental long-term project.
Cube-sphere terrain, quadtree LOD, deterministic generation, camera-relative coordinates, and `wgpu` are established techniques that fit together well.

The full collection of advanced goals is too large to treat as one fixed release.
Weather simulation, volumetric clouds, erosion, detailed vegetation, browser support, and Earth data are each substantial projects.
Keeping them optional prevents them from obscuring progress on the core renderer.

The main technical risks are:

- cracks and visible transitions between LOD levels
- precision and camera control across planetary scales
- terrain generation latency and streaming stalls
- GPU resource lifetime and memory management
- feature differences across native WebGPU backends
- coupling simulation work too early to rendering architecture

These risks should be reduced with small prototypes, debug views, reproducible test planets, and performance measurements.

## 11. Success Criteria

The minimum successful version will:

- generate deterministic planets
- render a cube-sphere with dynamic terrain LOD
- support stable travel from orbit toward the surface
- stream terrain within bounded memory
- expose useful debug views and performance metrics
- run as a native Rust and `wgpu` application

A mature version may also include:

- improved terrain, oceans, and atmosphere
- GPU compute generation
- climate, biomes, and vegetation
- saving, sharing, and regional export
- clouds, weather, and seasons
- a browser demo
- real Earth data

The project does not require every mature feature to be valuable or complete.
