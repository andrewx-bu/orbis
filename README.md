# Orbis

Orbis is a procedural planet renderer and simulation project written in Rust.
The current native demo renders a directionally lit, perspective-projected, depth-tested cube sphere with deterministic CPU-generated terrain.
Terrain starts in automatic mode, selecting subdivision levels 0–2 independently for each patch using estimated screen-space geometric error.
Selection responds to camera movement and window size, with separate split and merge thresholds to reduce rapid switching near a boundary.
Drag with the left mouse button to orbit the cube sphere.
Use the mouse wheel or trackpad to zoom.
Press `D` to toggle between normal lighting and an unlit debug view with stable patch colors and boundary lines.
Debug colors indicate subdivision level: blue for level 0, green for level 1, and orange for level 2, with small color variations between patches.
Press `L` to toggle between automatic and manual subdivision.
Manual mode remembers its previous level, initially level 1.
Press `[` to decrease the remembered manual level or `]` to increase it; either key switches to manual mode.
Manual levels 0, 1, and 2 render 6, 24, and 96 patches respectively, each with a 32×32 grid of cells.
Manual subdivision changes apply uniformly across the planet.
Each patch includes an inward-extending skirt along its boundary to cover gaps when neighboring patches use different subdivision levels.

The window title shows the current mode, active level range, and active and cached patch counts.
Cached patches include active patches and remain available when switching levels.
The cache holds at most 126 patches across all three levels.
Newly selected patches generate their meshes synchronously and may briefly pause rendering.
Skirts hide gaps between levels, but geometry may still visibly pop when detail changes.

See [PLAN.md](PLAN.md) for the project goals and roadmap.
