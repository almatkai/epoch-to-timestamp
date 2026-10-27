# Epoch to Timestamp for Tezbar

Convert Unix epoch (seconds or milliseconds) from clipboard to a human-readable timestamp in Tezbar on Windows or macOS.

## Native V2 (Rust)

The standalone [Native V2 library](v2/README.md) preserves the clipboard conversion rules and original icon. It uses the existing Tezbar beta API and requires host registration and an app rebuild. Real macOS/Windows V2 UI verification remains `not-run`; the V1 package below is preserved.

## Command

- **Epoch** — reads numeric clipboard content; if value ≥ 2^31 treats as milliseconds, otherwise seconds; shows `date.toString()` in an alert with Copy action. Invalid clipboard shows failure toast.

## Compatibility

- Windows: built and exercised in Tezbar on Windows 11.
- macOS: uses only cross-platform `@raycast/api` (pure JS Date) and retains the upstream macOS-compatible design.
- Linux: not currently declared.

## Local verification

```powershell
pnpm install
pnpm typecheck
pnpm test
pnpm build
```

## Credits

Adapted for Tezbar from Raycast's MIT-licensed `epoch-to-timestamp` extension at upstream commit `6734d85992290066fd31a7f5bee3cca3d3545a1a`. Original author `bgreenlee` remains credited in `package.json`.
