# Orbis

Orbis is a procedural planet renderer and simulation project written in Rust.
The current native demo renders a directionally lit, perspective-projected, depth-tested cube sphere with deterministic CPU-generated terrain.
Terrain is divided into 24 fixed patches, four per cube face.
Drag with the left mouse button to orbit the cube sphere.
Use the mouse wheel or trackpad to zoom.
Press `D` to toggle between normal lighting and an unlit debug view with stable patch colors and boundary lines.

See [PLAN.md](PLAN.md) for the project goals and roadmap.
