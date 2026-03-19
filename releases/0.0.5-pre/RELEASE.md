# rey v0.0.5-pre
Date: March 19, 2026

## What's new
- String and Mixed Types auto-conversions (e.g. `"HP: " + 100`).
- String interpolation natively built-in! (e.g. `"HP: {playerHp}"`).
- Global `print()` without trailing newlines.
- Global `println()` now accepts multiple arguments.
- Standard evaluators now include `.toString()`, `.toInt()`, and `.toFloat()`.
- Standard temporary built-ins added: `abs()`, `max()`, `min()`, `random()`.
- Added support for `const` to declare immutable variables statically.
- Unannotated variables (`var x = 10;`) are now Dynamically Typed (`Ty::Any`) and can mutate their types upon reassignment!
- Error messages completely revamped with a Rust/Miette-like visual diagnostic display pointing directly to the faulty source code lines and columns!
- Clean compiler output with all warnings completely resolved.

## What's fixed
- Added missing lexer support for `break` and `continue` keywords preventing proper nested short-circuiting.
- Relaxed TypeChecker strictness allowing `push`, `pop`, `len`, indexing, property access and arbitrary expressions to gracefully support flexible dynamic variable types.
- Fixed division (`/`) behavior correctly promoting to float precision across int combinations.

## Known limitations
- Modulo operator (`%`) is tokenized but not parsed.
- Compound assignment (`+=`, `-=`, etc) is not implemented.
- Increment/decrement (`++`, `--`) is not implemented natively yet.

## How to install and run
1. Use the binaries in this folder:
- `rey-v0-macos-arm64`
- `rey-v0-windows-x86_64.exe`

2. Run a Rey file:
- macOS arm64:
  `./rey-v0-macos-arm64 ../../compiler/v1/src/tests/full_demo.rey`
- Windows x86_64:
  `rey-v0-windows-x86_64.exe ..\\..\\compiler\\v1\\src\\tests\\full_demo.rey`

## Example programs
```rey
func main(): Void {
    const MAX_LIVES = 3;
    var name = "Hero";
    var hp = 100;

    println("Welcome {name}! You have {hp} HP.");
    
    // Dynamic typing in action
    var dynamic = 50;
    dynamic = "Now a string!";
    println(dynamic);
    
    // String Methods
    var value = "3.14".toFloat();
    println("Math random: ", random());
}
```
