# std::linalg

Pure Rey linear algebra helpers for 2D and 3D floating-point vectors.

## Types and functions

- `Vec2 { x: float, y: float }` and `Vec3 { x: float, y: float, z: float }` are public vector structs.
- `vec2(x, y)` and `vec3(x, y, z)` construct vectors.
- `vec2Add`, `vec2Sub`, `vec2Scale`, and `vec2Dot` provide 2D arithmetic; `vec2Length`, `vec2Distance`, and `vec2Normalize` provide common measurements.
- `vec3Add`, `vec3Sub`, `vec3Scale`, `vec3Dot`, and `vec3Cross` provide 3D arithmetic; `vec3Length` and `vec3Normalize` provide common measurements.

Normalization returns the zero vector when given the zero vector. Lengths use the compiler's `nativeSqrt` built-in.

```rey
import std::linalg;

var point = linalg.vec2(3.0, 4.0);
var unit = linalg.vec2Normalize(point);
var distance = linalg.vec2Distance(point, linalg.vec2(0.0, 0.0));
```
