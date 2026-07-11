# Windows ARM64 Build Contract Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Turn the proven Windows 11 ARM64 Whisper recipe into a repeatable, fail-closed repository build command.

**Architecture:** Add one PowerShell entry point that discovers or accepts a Visual Studio Build Tools installation, validates every ARM64 LLVM/MSVC file, imports the native developer environment, and builds Kaydence with target-generated Whisper bindings. Exercise its discovery contract with a filesystem fixture on every Windows CI leg; the real ARM64 VM remains the native execution authority.

**Tech Stack:** Windows PowerShell 5.1+/PowerShell 7, Visual Studio Build Tools 2022, clang-cl, Ninja, libclang, Rust/Cargo, pnpm, GitHub Actions.

## Global Constraints

- Windows ARM64 requires `Microsoft.VisualStudio.Component.VC.Llvm.Clang` and `Microsoft.VisualStudio.Component.VC.Llvm.ClangToolset`.
- Build with `GGML_NATIVE=OFF`, `CXXFLAGS=/EHsc`, Ninja, and native ARM64 `clang-cl`/`libclang`.
- `WHISPER_DONT_GENERATE_BINDINGS` must be absent; incompatible bundled glibc bindings must never enter a Windows ARM64 release.
- The real build must use Rust host `aarch64-pc-windows-msvc`; Windows PowerShell 5.1 may itself run as x64 under ARM64 Windows and is not a build-target authority.
- No dependency, network-allowlist, model-registry, UI invariant, or installer-format change belongs in this unit.
- Keep the unit within the repository's 400-line review limit and save runtime proof under `ops/mission/evidence/`.

---

### Task 1: PowerShell Contract Test

**Files:**
- Create: `tests/windows-arm64-build-contract.ps1`
- Modify: `.github/workflows/ci.yml`

**Interfaces:**
- Consumes: `scripts/windows-arm64-build.ps1 -BuildToolsPath <path> -Check`
- Produces: a Windows CI assertion over `architecture`, required component IDs, LLVM/Ninja/linker paths, `ggml_native`, and `generate_bindings`

- [ ] **Step 1: Write the failing filesystem-fixture test**

Create a temporary Build Tools tree containing `VsDevCmd.bat`, ARM64 `clang-cl.exe`, `libclang.dll`, Ninja, and an ARM64 MSVC linker. Invoke the production script in `-Check` mode, parse its JSON, and assert:

```powershell
$contract.architecture | Assert-Equal "aarch64-pc-windows-msvc"
$contract.ggml_native | Assert-Equal "OFF"
$contract.generate_bindings | Assert-Equal $true
$contract.required_components.Count | Assert-Equal 2
```

Delete `clang-cl.exe`, invoke again, and require failure text containing both official component IDs.

- [ ] **Step 2: Run the test and verify RED**

Run on Windows:

```powershell
powershell.exe -NoProfile -ExecutionPolicy Bypass -File tests/windows-arm64-build-contract.ps1
```

Expected: FAIL because `scripts/windows-arm64-build.ps1` does not exist.

- [ ] **Step 3: Add the test to the existing Windows harness-contract CI step**

Parse both production PowerShell scripts with `System.Management.Automation.Language.Parser`, then invoke:

```powershell
./tests/windows-arm64-build-contract.ps1
```

Expected: CI keeps running this on every Windows leg, including ordinary three-OS PR matrices.

### Task 2: Native ARM64 Build Entry Point

**Files:**
- Create: `scripts/windows-arm64-build.ps1`
- Modify: `scripts/AGENTS.md`

**Interfaces:**
- Consumes: optional `-BuildToolsPath`, `-Profile release|debug`, and `-Check`
- Produces: JSON toolchain contract in `-Check`; otherwise `target/<profile>/kaydence.exe`

- [ ] **Step 1: Implement fail-closed toolchain discovery**

Use an explicit path first, then `vswhere.exe`, then conventional Build Tools locations. Validation must resolve these exact files before any build starts:

```text
Common7\Tools\VsDevCmd.bat
VC\Tools\Llvm\ARM64\bin\clang-cl.exe
VC\Tools\Llvm\ARM64\bin\libclang.dll
Common7\IDE\CommonExtensions\Microsoft\CMake\Ninja\ninja.exe
VC\Tools\MSVC\*\bin\Hostarm64\arm64\link.exe
```

Any missing file throws one actionable error naming both required Visual Studio component IDs.

- [ ] **Step 2: Implement deterministic check output**

`-Check` must perform path validation without requiring an ARM64 host and emit only this JSON contract:

```json
{
  "architecture": "aarch64-pc-windows-msvc",
  "ggml_native": "OFF",
  "generate_bindings": true,
  "required_components": [
    "Microsoft.VisualStudio.Component.VC.Llvm.Clang",
    "Microsoft.VisualStudio.Component.VC.Llvm.ClangToolset"
  ]
}
```

Include absolute resolved paths for Build Tools, `VsDevCmd`, clang, libclang, Ninja, and link.

- [ ] **Step 3: Implement the native build path**

The non-check path must:

1. Reject non-`aarch64-pc-windows-msvc` Rust hosts; do not infer the build target from the PowerShell process architecture.
2. Reject a non-empty `WHISPER_DONT_GENERATE_BINDINGS`.
3. Import `VsDevCmd.bat -arch=arm64 -host_arch=arm64` through a temporary command file.
4. Prepend native LLVM and Ninja paths.
5. Set `CMAKE_GENERATOR=Ninja`, all three CMake compiler variables to ARM64 `clang-cl`, `CXXFLAGS=/EHsc`, `LIBCLANG_PATH=<LLVM bin>`, and `GGML_NATIVE=OFF`.
6. Run `pnpm --filter kaydence-desktop build`.
7. Clean only `whisper-rs-sys` for the selected profile, then run Cargo with `custom-protocol,asr-whisper`.
8. Fail on every non-zero native command and verify `target/<profile>/kaydence.exe` exists.

- [ ] **Step 4: Run the fixture test and verify GREEN**

```powershell
powershell.exe -NoProfile -ExecutionPolicy Bypass -File tests/windows-arm64-build-contract.ps1
```

Expected: `PASS: Windows ARM64 build contract is fail-closed`.

- [ ] **Step 5: Run repository static gates**

```bash
bash scripts/check-frontend.sh
bash scripts/audit-network.sh
bash scripts/check-privacy-posture.sh --check
bash scripts/check-adr-status.sh
```

Expected: all pass; no network surface or ADR status changes.

### Task 3: Real Windows ARM64 Proof And Mission Heartbeat

**Files:**
- Create: `ops/mission/evidence/2026-07-11-p1-g3-windows-arm64-build-contract.txt`
- Modify: `ops/mission/state.json`
- Modify: `ops/mission/journal.md`
- Regenerate: `ops/mission/state-of-the-union.html`

**Interfaces:**
- Consumes: the script from Task 2 and the existing Windows 11 Pro ARM64 VM
- Produces: inspectable toolchain/build evidence and a truthful mission checkpoint

- [ ] **Step 1: Run check mode in the real VM**

```powershell
powershell.exe -NoProfile -ExecutionPolicy Bypass -File scripts/windows-arm64-build.ps1 -BuildToolsPath C:\BuildTools -Check
```

Expected: JSON names the native ARM64 clang, Ninja, libclang, and linker paths; `generate_bindings` is `true`.

- [ ] **Step 2: Run the real release build**

```powershell
powershell.exe -NoProfile -ExecutionPolicy Bypass -File scripts/windows-arm64-build.ps1 -BuildToolsPath C:\BuildTools -Profile release
```

Expected: exit 0 and `target\release\kaydence.exe` exists. Capture tool versions, source SHA, command output, binary SHA256, and binary architecture in the evidence file. If disk or tool state prevents completion, record the exact failure and leave release automation unverified.

- [ ] **Step 3: Render and validate the mission heartbeat**

Update only the Windows ARM64 release-automation checkpoint, append the journal, then run:

```bash
node ops/mission/render-sotu.mjs
jq empty ops/mission/state.json
git diff --check
```

Expected: valid JSON, regenerated State of the Union, no whitespace errors, and no claim that P1-G3 or installer packaging is complete.

- [ ] **Step 4: Commit the independently reviewable unit**

```bash
git add scripts/windows-arm64-build.ps1 scripts/AGENTS.md tests/windows-arm64-build-contract.ps1 .github/workflows/ci.yml ops/mission/evidence/2026-07-11-p1-g3-windows-arm64-build-contract.txt ops/mission/state.json ops/mission/journal.md ops/mission/state-of-the-union.html
git commit -m "build(p1): automate Windows ARM64 Whisper releases"
```

Expected: one commit whose evidence distinguishes contract validation, native build proof, and still-open P1/P3 gates.
