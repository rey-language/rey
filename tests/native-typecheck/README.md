# Native typechecker focused cases

Each `.rey` under `positive/` should pass `check`; each `.rey` under `negative/` should fail and contain the substring in its sibling `<basename>.expect_check_fail` file. These fixtures exercise the native checker only, not the stale committed `rey-compiler/main` binary. Run them with a compiler rebuilt from the current source tree and an alarm guard around each invocation.

`positive/checked-handoff.rey` is the single-unit checked-lowering input. It covers a direct call, struct field lookup, and typed HashMap `get`.

`positive/checked-wrapper-metadata.rey` is an executable source assertion for outer tagged-expression identity, typed call/field/map receiver records, field initializer payload plus wrapper, cross-module owner metadata, and private namespace diagnostics. Run it as a program with the Rust oracle after its host checker accepts the current source API.

`positive/nominal-layout-owners.rey` is an executable assertion for owner-qualified struct and enum layouts, nested nominal types inside builtins, nullable, tuple, and function types, distinct same-named declarations, and imported private-field rejection. Run it the same way, with `rey-compiler/src` importable.

`graphs/<case>/main.rey` fixtures exercise retained import edges, module namespace/selected bindings, diamond deduplication, public calls, private declarations, import-name collisions, and tagged-wrapper public type/field resolution. Each graph case is a separate entry directory; siblings are the imported source units.

`graphs/same-name-nominal/` declares `Point` and `Event` in two imported modules, with different fields and payloads. `Holder` nests a nominal, `Vec`, `HashMap`, `Option`, `Result`, nullable, tuple, and function type. The entry reads a nominal field whose type is owned by a third module.

`graphs/private-field/` imports a public struct and must still reject access to a private field.

Graph fixtures exercise typechecking and import ownership; this suite does not assert multi-unit native lowering.

Coverage: inferred and annotated locals/returns, function signatures and arity, bool conditions, const assignment, nominal struct fields/literals, Vec-vs-HashMap receiver typing, null/`instanceof` narrowing, immutable lambda captures, explicit string concatenation, renderable variadic stdout/stderr printing, enum payloads and match exhaustiveness including bare variants, full payload-wrapper dispatch, recursively irrefutable nested payloads, refutable payload diagnostics, imported span-helper graphs, unknown identifiers/types, and canonical primitive spellings.
