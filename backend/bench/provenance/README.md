# Run provenance

`frozen-build.json` records the compiler source and executable hashes used for the current corpus run. The per-program archived ELFs and reports are under `../artifacts/`; old compiled pairs and their exact corpora stay in `../history/round-two/programs/`. Intermediate cohort runs live under `../history/round-three/`.

`../gaps/round-three/performance/` compares the old and new Tare ELFs against those unchanged round-two corpora. The native RunReport bytes are retained unchanged beside separately serialized comparison metadata, including each report hash.

No image, validator, or full mainnet sampling result is inferred from these native runs. The native RPC rehearsal states its own compiler and registry ELF hashes separately.
