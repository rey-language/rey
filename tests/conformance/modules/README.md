# Module conformance cases

These cases follow `SPEC.md` §7. Positive outputs are byte-exact and use only
local helper files. The private-access negatives encode the target visibility
rule: private declarations are module-local. In particular, the current v1
resolver flattens imported declarations, so bare-name private access and
same-name private helpers across modules remain known gaps until import-owner
visibility metadata reaches typechecking.
