# Rey — GitHub Linguist submission prep

This repo contains **Rey**, an experimental programming language and interpreter written in Rust.

This directory layout is intended to prepare a future PR to `github-linguist/linguist` so GitHub can recognize `.rey` files as the **Rey** language (syntax highlighting on GitHub, language stats, etc.).

## Repo links
- Rey language repo: https://github.com/rey-language/rey
- GitHub Linguist: https://github.com/github-linguist/linguist

## Proposed Linguist metadata
- `.gitattributes` marks `*.rey` as `linguist-language=Rey`
- `languages/Rey.yaml` contains the language definition (format mirrors Linguist language YAML entries)
- `languages/samples/Rey.rey` is a clean consolidated sample for tests/highlighting

## Sample sources
The sample is consolidated from existing test fixtures:
- `compiler/v1/src/tests/arrays.rey`
- `compiler/v1/src/tests/array_methods.rey`
- `compiler/v1/src/tests/functions.rey`
- `compiler/v1/src/tests/property_access.rey`
- `compiler/v1/src/tests/full_demo.rey`

## Sample code
```rey
// see languages/samples/Rey.rey for the full sample
func main(): Void {
    var xs: [int] = [];
    xs.push(1);
    println(xs.length());
}
```

## How this maps to a future Linguist PR
In `github-linguist/linguist`, this would translate to:
- Adding `.rey` + metadata to Linguist’s language definitions
- Adding `languages/samples/Rey.rey` (or equivalent) as the sample used for detection/highlighting tests

This repo keeps the metadata and sample here to make the upstream PR straightforward.
