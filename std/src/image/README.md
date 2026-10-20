# `std::image`

`std::image` stores RGB pixels in a row-major `Vec<int>` and serializes images
as plain-text PPM (P3).

## API

- `Image { width: int, height: int, pixels: Vec<int> }` stores the dimensions
  and one packed 24-bit RGB integer per pixel.
- `createImage(width: int, height: int): Image` creates a black image.
- `setPixel(img: Image, x: int, y: int, r: int, g: int, b: int): void` sets a
  pixel; coordinates must be in bounds and channel values must be 0–255.
- `getPixel(img: Image, x: int, y: int): Vec<int>` returns `[r, g, b]`.
- `toPpm(img: Image): string` returns a P3 header and decimal RGB values.
- `writePpm(img: Image, path: string): bool` writes the PPM text to a file and
  reports whether the path exists afterward.

This module does not decode images or write binary bitmap formats. Its file
writer uses the compiler's filesystem built-ins and therefore inherits host
filesystem permissions and path behavior.
