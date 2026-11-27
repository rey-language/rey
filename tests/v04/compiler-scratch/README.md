# Compiler scratch lifetime acceptance

The named bounded pattern is `boundedScratch`: one fixed 8-byte allocation in
a repeated CFG block, observed only by raw equality in that block. It has no
unknown calls, stores, derived addresses, or values crossing a block edge.
The acceptance builds IR and managed-function counts around 511/512/513,
checks destination construction and conservative fallback cases, then
independently tampers with completion plans to confirm the verifier rejects
them. The acceptance builds the analysis, region-lowering, and runtime
measurement drivers separately. The runtime probe stages the transitive
function closure of the exact layout, managed, and region APIs it calls;
`stage-runtime.py` copies those function bodies verbatim from `$SRC` and
fails if a reachable function or supported dependency is missing.

The runtime measurement warms a kind-1 cohort, records the maximum
`managedRegionScratchUsed` for one iteration, then for ten iterations. Each
iteration allocates the fixed payload and ends its cohort before continuing.
The observed peak must remain within 1.1× of the warmed one-iteration peak.
`run.sh` also runs the staged R01 suite using `r01-uncapped.py`, which holds
`/home/ubuntu/rey/build.lock` for the complete gate and records `/usr/bin/time
-v` peak RSS for each compiler invocation without a memory cap or memory
availability abort.
