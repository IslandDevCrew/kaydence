# settings/ — Configuration Truth

## Owns
The typed Settings struct (single source of truth), schema-versioned JSON on
disk, hot-reload broadcast to modules, BYOK secret storage (OS keychain via
keyring — **never plaintext on disk, never in the settings file**), the
`APP_NAME` constant (working title "Kaydence" — brand may change, nothing else
hardcodes it), and first-run defaults.

## Invariants
1. Defaults embody the product thesis: Light dial, push-to-talk, local CPU
   engine, telemetry nonexistent (there is no flag because there is no
   telemetry), retention 30 days.
2. Every setting has: type, default, validation, and a UI surface or an
   explicit "advanced/json-only" tag. No dead settings.
3. Settings file is human-readable JSON with a `schema_version`; unknown keys
   warn, never crash (forward compat).
4. Secrets API exposes set/clear/exists only — no read-back to the frontend.
