# Fixture provenance

`beta_common.rs`, `extension-v2.schema.json` and `TEZBAR-LICENSE.txt` are
**unmodified** snapshots from Tezbar beta commit
`fd300e276356587726566026f072563eb9c34cb1`, obtained read-only with `git show`.
The host sources are Apache-2.0; their original LICENSE is retained here.
No fixture is relabeled MIT. Package SPDX expression is `MIT AND Apache-2.0`.

| Fixture | Pinned host path | SHA-256 |
| --- | --- | --- |
| `beta_common.rs` | `src-tauri/src/native_core/common.rs` | `280a0a2f0dd04a8570b320aafc0c3ece0f5de8ddb21c363645567d6a35d84da7` |
| `extension-v2.schema.json` | `schemas/extension-v2.schema.json` | `6d5afda1b652605982104bdf16d1547547b98dfd707a7b2969316406c49f32af` |
| `TEZBAR-LICENSE.txt` | `LICENSE` | `0ed6b1b2760294d4aaaa45be38e166fcc0168e1dd5190423c096fcbe1c94afbf` |

The contract harness includes the real `common.rs` and the unchanged production
`tezbar_adapter.rs`. It substitutes only Tauri AppHandle/Emitter and injects a
recording router. No real OS clipboard/window actions occur. The original
common.rs's two configuration tests also run against temporary test directories.

The following MIT V1 source is pinned to this extension repository commit
`59ef35503dbf44c36326f771cf102b6276adc1da`:

| Source or fixture | SHA-256 |
| --- | --- |
| `src/epoch.tsx` | `e7757dcb6e0c9a7e61002f97f7c0f0e28b59ec1116b4edb17dc8dd1f4289df6c` |
| `src/lib/epoch.ts` | `4812c5110d92b220d44a2bef86413ad40e0f9be225603e4134a7abff98d0fa90` |
| `v1-package.json` (unchanged package snapshot) | `10f651f46f2470d85376d98ff4875baf1e91e7cfee24dcf6e567f03cf6e864b9` |
| `v1-vectors.json` | `c787f9e88414345f08318cc820d77dd223aa8509ba388f700cd8ae2d6be8cc75` |

`v1-vectors.json` was generated with `../generate_v1_vectors.mjs`, which imports
V1 `epochToDate` and `epochToString` directly using Node's built-in TS stripping.
It records Node/ICU/tzdata versions and source hashes. It includes 1280 UTC cases
and 72 local timezone cases (six named zones). Numeric conversion, invalid date
behavior and full UTC strings are exact comparisons. The local oracle separates
numeric wall time/GMT offset from implementation-defined parenthetical labels.
Rust tests use the checked-in JSON without requiring Node or the surrounding V1
checkout. Regeneration requires that checkout; do not generate expectations
from the Rust implementation.
