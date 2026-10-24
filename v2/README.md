# Epoch to Timestamp — Native V2

A standalone Rust **library** for compiling into Tezbar beta. It implements the
existing Native V2 API; it does not run an external process, load Rust from the
store, or modify an installed app. The V1 TypeScript, package/installer metadata,
assets and tests remain available in the repository root.

Host reference: Tezbar beta commit
[`fd300e276356587726566026f072563eb9c34cb1`](https://github.com/almatkai/Tezbar/tree/fd300e276356587726566026f072563eb9c34cb1),
read with `git show`: `docs/NATIVE_V2_FORMAT.md`, the exact beta schema,
`v2_runtime.rs`, `v2_sdk.rs`, `native.rs`, `v2_google_search.rs`, `common.rs`,
`platform.rs`, clipboard routing and Vite configuration. Package organization
and the injected Host/concrete CoreContext wrapper follow the
[published Base64 example](https://github.com/almatkai/base64/tree/f586814866962b21225cc6f870f585cc3787d224/v2).

## Behavior and deliberate UI adaptation

The `epoch` command reads the clipboard **once per launch** and applies the
actual V1 source rules, without a units heuristic:

- Convert the whole string with JavaScript `Number` semantics. Empty/whitespace
  text becomes `0`. Decimal, exponent, unsigned hexadecimal/binary/octal,
  ECMAScript whitespace and signed `Infinity` are supported. `NaN` produces
  `No number in the clipboard.` as a native error (V1 uses a failure toast).
- Values **below `2^31` are seconds**, including negative values and fractions;
  multiply as a double by 1000. Values at/above the boundary are milliseconds.
  For example `2208988800` becomes January 1970, while `2208988800000` is 2040.
- Apply JS Date TimeClip: truncate milliseconds toward zero, with valid range
  **±8,640,000,000,000,000 ms**. Nonfinite or out-of-range values display and copy
  **`Invalid Date`**, just as V1; they are not validation errors.
- Render the date with local calendar fields and GMT offset. No millisecond
  field is added. Copy writes the exact displayed string.

Beta's Native V2 SDK has no Raycast `confirmAlert` interface. The result uses its
existing `Detail` root with `props.markdown`, `props.actionIds`, and top-level
`actions`: **Copy to clipboard**, then **Close**. Copy awaits clipboard success
and keeps the detail visible; Close awaits `window:hide` and ends the view. This
replaces V1's modal alert; no extra screen or settings are added. A host failure
returns an error, keeps the copy/close target retryable and produces no success
response. Both `Err` and `{ok:false,message/error}` failure forms propagate.

## Dates and local timezone differences

`epoch::epoch_millis` and `epoch::format_date` accept explicit timezone data for
reproducible tests. Calendar arithmetic covers the **entire JS Date range**,
including year `-271821` through `275760`; it does not use Chrono's narrower
calendar. The boundaries are checked against Node V1 output.

`local::format_local` uses the real runtime local timezone at the target instant:
Unix `localtime_r`/`strftime`, or Windows dynamic timezone rules and bias. Offset
seconds are retained for historical wall times (e.g. Paris's historical
`00:09:21`); the displayed `GMT±HHMM` field truncates offset seconds like JS.
The English day/month/date layout is deterministic.

**Specific text difference:** on Unix UTC the OS supplies `(UTC)`, while Node's
ICU V1 oracle supplies `(Coordinated Universal Time)`. New York similarly uses
`(EST)`/`(EDT)` rather than ICU's expanded names. Windows supplies its OS timezone
name, which can be localized. The timezone name is implementation-defined in
[ECMAScript Date.toString](https://tc39.es/ecma262/multipage/numbers-and-dates.html#sec-date.prototype.tostring);
byte parity for that parenthetical is not claimed.

Numeric wall time and GMT offset follow the installed **OS timezone database**,
including DST and historical second offsets, rather than an embedded ICU copy.
The Unix helper is tested against Node in UTC, New York, Kathmandu, Paris,
Sydney and Almaty, including DST edges and Almaty's 2024 offset change. Timezone
database revisions can differ between the OS and JS ICU. On Windows the native
historical-rule API covers years 1601–30827: outside that interval the endpoint's
recurring rules are evaluated in the original year, preserving the full calendar
range. This is OS-rule extrapolation, not a claim that ICU has identical distant
historical/future rules. Non-recurring Windows rules and OS lookup failures
return concrete errors instead of silently substituting UTC. Windows runtime
results have not been exercised on this machine.

## Build and verification

From `v2/`:

```sh
cargo build --locked
cargo fmt --check
cargo clippy --all-targets --locked -- -D warnings
cargo test --locked
cargo +1.85.0 test --locked
cargo package --allow-dirty
```

`rust-version` is 1.85. `Cargo.lock` is resolved for that MSRV; retain it and review
updates. Runtime dependencies are `serde_json` plus Unix `libc` or Windows
`windows-sys`. Schema/hash/async-router tools are dev dependencies. Tests run
from the packaged crate without the V1 checkout, Node, Tauri, or an installed app.
The explicit package include list and `.gitignore` exclude build/dependency output.

The oracle fixture has **1280 UTC vectors and 72 local vectors**, generated from
untouched V1 with Node 26.7.0. Tests compare IEEE-754 Number bits, threshold,
TimeClip, invalid dates, complete UTC strings, explicit local offsets, and the
actual Unix helper in isolated TZ child processes. Pure Windows DST-rule tests
cover northern/southern transitions and extreme years. The concrete, unchanged
`tezbar_adapter.rs` is also compiled with the **real pinned beta `common.rs`**;
only Tauri AppHandle/Emitter and the OS-effect router are substituted.
Other tests cover manifest/schema metadata, original icon hash, Copy/Close,
refresh without rereading, stale/forged actions, failed relaunch, dispose, and
host failures. See [fixture provenance](tests/fixtures/PROVENANCE.md).

V1 ships `package-lock.json`, not a pnpm lockfile. To verify with pnpm while
preserving that installer, use a disposable directory **inside this clone**:

```sh
# From the repository root:
mkdir -p v2/target/v1-check
cp package.json package-lock.json tsconfig.json raycast-env.d.ts v2/target/v1-check/
cp -R src test v2/target/v1-check/
cd v2/target/v1-check
pnpm import
pnpm install --frozen-lockfile --ignore-scripts
pnpm test
pnpm typecheck
```

Optional oracle regeneration from the root uses Node >=22.18's built-in TS
stripping and imports the actual V1 library (no npm dependencies):

```sh
node v2/tests/generate_v1_vectors.mjs
```

**Not run:** full Tezbar build, real macOS/Windows UI, real clipboard access,
permissions, focus/window behavior, or Windows timezone APIs. Both manifest
verification entries remain `not-run`. Fake-router success is not OS QA.

## Adapter contract

Exports: `commands()` (`["epoch"]`), `ICON` (original PNG bytes), `MANIFEST`,
`Host` and async `handle(host, command, operation, payload, state)` returning
`Result<serde_json::Value, String>`. `tezbar_adapter.rs` wraps
`CoreContext::request` with a Send boxed future and exposes the concrete beta
`handle(ctx, command, operation, payload, state)` signature.

- `extension:run-command`: initialize a new revision, read clipboard, convert,
  capture the displayed/copyable string. Even a failed relaunch invalidates old actions.
- `extension:invoke-action`: resolve `payload.actionId` against current session
  revision. Ignore supplied content. Reject unknown, stale and inactive actions.
- `extension:refresh-session`: return the captured detail with unchanged IDs;
  never reread clipboard or recalculate timezone text.
- `extension:dispose-session`: clear result and invalidate actions; idempotent.
- Search/load-more and unknown commands/operations are rejected: this detail
  has no search or pagination.

The real host owns session IDs, serialization, cancellation, installed-command
checks and preferences. V1 declares no preferences. State is session-local;
there are no files, timers, worker processes or background side effects.

| Effect | Actual beta route | Positional args |
| --- | --- | --- |
| Read text | `clipboard:read` | `[]` → string |
| Copy displayed result | `clipboard:write` | `[text]` |
| Close | `window:hide` | `[]` |

## Host integration (requires source changes and app rebuild)

These are integration instructions; this crate has not applied them to any app.
The canonical ID is `com.tezbar.native.epoch-to-timestamp`, replacing
`raycast.epoch-to-timestamp`. A manifest/marker alone cannot execute Rust code.

1. Vendor this whole crate, e.g. at
   `src-tauri/native-extensions/epoch-to-timestamp`. Preserve licenses and fixtures.
   Add the path dependency to the host's `src-tauri/Cargo.toml`:

   ```toml
   tezbar-epoch-to-timestamp-v2 = { path = "native-extensions/epoch-to-timestamp" }
   ```

   Update/review the host's own Cargo.lock. Copy `tezbar_adapter.rs` unchanged to
   `src-tauri/src/native_core/extensions/v2_epoch_to_timestamp.rs`.

2. Register `mod v2_epoch_to_timestamp;` in `native_core/extensions/mod.rs`.
   Add `"epoch-to-timestamp"` to `v2_runtime::slug`'s allowlist, then add these
   arms to `commands` and `handle`, respectively:

   ```rust,ignore
   "epoch-to-timestamp" => super::v2_epoch_to_timestamp::commands(),
   "epoch-to-timestamp" => super::v2_epoch_to_timestamp::handle(ctx, command, operation, payload, state).await,
   ```

   Keep beta's session and installed-command checks.

3. Append the **object** from `manifest.json` to
   `src-tauri/src/native_core/extensions/v2_catalog.json`; validate against the
   exact beta schema. Update catalog-count/coverage tests for the new command.
   Metadata preserves V1's `mode: "no-view"`; the adapter's returned `mode: "view"`
   creates the native detail session. The schema disallows a top-level author
   field, so credits are in `description` instead.

4. Copy `assets/extension-icon.png` byte-for-byte to
   **`src/renderer/public/extensions/icons/epoch-to-timestamp.png`** in the host.
   Beta's Vite `publicDir` is `src/renderer/public`, not root `public`.
   Manifest URL must remain **`/extensions/icons/epoch-to-timestamp.png`**.
   See [asset provenance](assets/PROVENANCE.md); no resizing/recompression.

   Beta's installer writes JSON only. If managed-package icon fallback is
   required, write the embedded original before recording successful installation
   in `v2_catalog::install`:

   ```rust,ignore
   if slug == "epoch-to-timestamp" {
       std::fs::create_dir_all(root.join("assets")).map_err(|e| e.to_string())?;
       std::fs::write(
           root.join("assets/extension-icon.png"),
           super::v2_epoch_to_timestamp::ICON,
       ).map_err(|e| e.to_string())?;
   }
   ```

   Resolve that fallback explicitly under managed canonical
   `extensions/packages/com.tezbar.native.epoch-to-timestamp/assets/extension-icon.png`.
   Preserve V1 assets and the schema-required absolute catalog icon URL.

5. Rebuild/test the host and inspect its compiled Native V2 coverage CLI. After
   registration/rebuild, install the canonical ID with the existing Native V2
   catalog install route. The store cannot directly install this repo's Rust.
   Preserve the existing V1 package for fallback.

6. On macOS and Windows separately, test blank/numeric/invalid clipboard,
   seconds/ms boundary, negative/fractional values, `Infinity`, local DST,
   Copy/Close, permission failures and session disposal. Record OS QA evidence
   before changing verification from `not-run`.

## Credits and licensing

Original MIT Raycast extension: **bgreenlee**. Tezbar adaptation: **almatkai**.
Behavior comes from this repository's V1 commit
`59ef35503dbf44c36326f771cf102b6276adc1da`; original Raycast source/icon pin:
[`6734d85992290066fd31a7f5bee3cca3d3545a1a`](https://github.com/raycast/extensions/tree/6734d85992290066fd31a7f5bee3cca3d3545a1a/extensions/epoch-to-timestamp).
The MIT text is retained in `LICENSE`. Beta fixtures retain their Apache-2.0
license in `tests/fixtures/TEZBAR-LICENSE.txt`; the distributed package is
`MIT AND Apache-2.0`, without relicensing those fixtures as MIT.
