# Orbis

Orbis is a procedural planet renderer and simulation project written in Rust.
The current native demo renders a directionally lit, perspective-projected, depth-tested cube sphere with deterministic CPU-generated terrain.
Terrain starts at subdivision level 1 with 24 patches, four per cube face.
Drag with the left mouse button to orbit the cube sphere.
Use the mouse wheel or trackpad to zoom.
Press `D` to toggle between normal lighting and an unlit debug view with stable patch colors and boundary lines.
Press `[` to decrease terrain subdivision or `]` to increase it.
Levels 0, 1, and 2 render 6, 24, and 96 patches respectively, each with a 32×32 grid of cells.
Subdivision changes apply uniformly across the planet.

The window title shows the current level and active and cached patch counts.
Cached patches include active patches and remain available when switching levels.
The cache holds at most 126 patches across all three levels.
The first visit to a level generates its meshes synchronously and may briefly pause rendering.

See [PLAN.md](PLAN.md) for the project goals and roadmap.
