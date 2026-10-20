# std::binary

Pure Rey byte and hexadecimal conversion helpers. Packed integers are unsigned; byte vectors contain integers in `0..255`.

## Functions

- `packU16Le` / `unpackU16Le` encode and decode unsigned 16-bit values in little-endian order.
- `packU32Le` / `unpackU32Le` encode and decode unsigned 32-bit values in little-endian order.
- `packU16Be` / `unpackU16Be` encode and decode unsigned 16-bit values in big-endian order.
- `packU32Be` / `unpackU32Be` encode and decode unsigned 32-bit values in big-endian order.
- `toHex(bytes)` converts bytes to lowercase hexadecimal text.
- `fromHex(hex)` parses even-length hexadecimal text, accepting either letter case.

Packing asserts when the value is outside the unsigned width. Unpacking asserts when the requested bytes are missing or any selected value is outside `0..255`. `fromHex` asserts on odd-length or non-hexadecimal input. An empty vector and empty string convert to one another.

```rey
import std::binary;

var bytes = binary.packU32Be(305419896);
var text = binary.toHex(bytes); // "12345678"
var value = binary.unpackU32Be(bytes, 0);
```
