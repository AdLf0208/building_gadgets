# Validation

> **Historical (0.1.0, the retired `.hnmod` format).** The current package is the
> SDK `.hmod` (**0.2.5**): its ladder, artifact and evidence are in
> [`notes.md`](notes.md) → `## Slice 4 — the buildinggadgets2:deny tag`, with the logs under
> `../_shared/evidence/buildinggadgets2/sdk-*.log` and the in-game legs under
> `../_shared/evidence/buildinggadgets2-e2e/`.
> **0.2.5 still holds the block writes** (`blocked: no-cancellable-placement`); the
> in-game proof is the held click, the two world-border edge phases and the three
> deny-tag phases, not a placed row. The staged `horizon.py` steps below no longer exist.

## 0.2.5 (the current package)

| Check | Result |
|---|---|
| `cargo test` | 41 passed, 0 failed (red first: 38 passed / 3 failed, `EXIT:101`) |
| `cargo clippy --all-targets -- -D warnings` / `cargo fmt --check` | exit 0 / exit 0 |
| `cargo horizon-mod build` twice (`rm -f Cargo.lock` each) | 43,350 B, `cmp` identical |
| `cargo horizon-mod check` / `fits --build 0a875da…` / `fits --build 0b14f8a…` | exit 0 / `made for` / `fits` |
| `verify-sdk-package.py buildinggadgets2` | 1/1 |
| `check-recipe-types.py buildinggadgets2` | `buildinggadgets2: ok` |
| `tools/e2e.sh` (modded, 14 phases) | `PASS` + `EXIT:0` |
| `tools/e2e.sh` with `GAME=$CONTROL_GAME` | `FAIL` + `EXIT:1` |
| `SELFTEST=1 tools/e2e.sh` | all 14 phases rejected (30 assertions) + `EXIT:1` |
| e2e with the destination read switched off (red) | `FAIL: deny_destination … 8 …, expected 6` + `EXIT:1` |
| `check-e2e-logs.py buildinggadgets2-e2e --strict` | 7/7 |

Artifact: `buildinggadgets2.hmod` 43,350 B, sha256
`9d19a3410c5dea90b062fb5818fea876710085ef412f9a23de151658c9bbfcf7`, 19 entries (the deny tag is the
new one), no packed `Cargo.lock`; declared version 0.2.5.

The staged commands used the slot registry (`build-slot.sh run --step test|lint|fmt|build|check|e2e`)
with `BUILD_LOCAL=1` (these steps take seconds in the SDK layout, so the remote box buys nothing), and
the in-game legs used the launcher-built game at the pin
(`launcher-game.sh build buildinggadgets2`, `Compiling buildinggadgets2 (1 of 1)`).

## 0.2.4 (previous)

| Check | Result |
|---|---|
| `cargo test` | 35 passed, 0 failed |
| `cargo clippy --all-targets -- -D warnings` / `cargo fmt --check` | exit 0 / exit 0 |
| `cargo horizon-mod build` twice (`rm -f Cargo.lock` each) | 40,798 B, `cmp` identical |
| `cargo horizon-mod check` / `fits --build 0a875da` / `fits --build 0b14f8a` | exit 0 / `made for` / `fits` |
| `verify-sdk-package.py buildinggadgets2` | 1/1 |
| `tools/e2e.sh` (modded) | `PASS` + `EXIT:0` |
| `tools/e2e.sh` with `GAME=$CONTROL_GAME` | `FAIL` + `EXIT:1` |
| `SELFTEST=1 tools/e2e.sh` | all 11 phases rejected + `EXIT:1` |
| `check-e2e-logs.py buildinggadgets2-e2e --strict` | 3/3 |

Artifact: `buildinggadgets2.hmod` 40,798 B, sha256
`927409d7750cc246e1e98630b6355bc9e90ba6c7cdf17c80f9e87bb05066f14a`, 18 entries, no
packed `Cargo.lock`.

Validated on Linux with Rust 1.95.0 against the exact revisions in
`compatibility.json` (Horizon `c9d474d`, horizon-mixins `d9c83d7`), 2026-11-05.

| Check | Result |
|---|---|
| `cargo fmt --check` | Pass |
| `cargo clippy --all-targets --locked --offline -- -D warnings` | Pass, 0 warnings |
| `cargo test --locked --offline -p horizon-buildinggadgets2` | 10 runtime tests pass, exit 0 |
| `python3 tools/horizon.py stage --horizon ../horizon` | Pass; full pinned export staged (`staging_seconds` 1587 under campaign load) |
| `cargo check` on the staged workspace | Pass, exit 0, 0 warnings (13m 17s) |
| `cargo test ... game::buildinggadgets2::tests` on the staged workspace | 10 passed, 0 failed, 531 filtered out, exit 0 |
| `cargo build` (fast) `horizon-client` and `horizon-ctl` | Pass, exit 0 (35m 32s / 9.62s) |
| `tools/e2e.sh` | Pass (fast binaries): 5 blocks placed, 5 stone consumed |
| `COUNTERFACTUAL=1 tools/e2e.sh` | Pass (fast binaries): 0 blocks placed, 0 stone consumed |
| `python3 tools/package_content.py` | 6034-byte `.hmod`, SHA-256 `5890eec2…d19049` |
| `cargo-horizon-mod horizon-mod check content/buildinggadgets2.hmod` | `{"ok":true}` |

The staged workspace's cargo commands are exactly the ones `tools/horizon.py
check/test/build` run; they were invoked directly because the wrapper re-runs
the 25-minute `plan`/`apply` before every action, which the campaign's 30–40
load average made impractical. The e2e used the `fast` binaries the build step
produced. Raw logs: `target/horizon-*.log` and the port's
`/tmp/opencode/buildinggadgets2-*.log`.

`tools/test_template.py` targets the template's trivial one-hook fixture; it
does not provide the Horizon `Game` API this port's support module uses, so it
is not a meaningful compile check here. The real pinned `check` above is. The
`test_stage.py` AST audit was not re-run after the pinned stage; the staging
itself (plan/apply) is what `horizon.py stage` performed.

The e2e uses scratch `HORIZON_HOME`, `XDG_DATA_HOME` and `XDG_CONFIG_HOME`
directories and the prepared 26.3 resources. No Minecraft assets or player data
are committed.

## 2026-11-09 — Horizon `1d63e0a`, both artifacts, e2e green

Validated against Horizon `1d63e0a5a37a7a708eb0b32b49bf0dfeb28ed8a9` +
horizon-mixins `5a64af938ec86b00d53d16a408aabc0038fb3e57`; every step ran
through `ports/tools/build-slot.sh` with `CARGO_INCREMENTAL=0`.

| Check | Result |
|---|---|
| `cargo fmt` + `--check`, `cargo clippy --all-targets -- -D warnings` | exit 0, 0 warnings |
| `cargo test --locked --offline` | 11 passed |
| `horizon.py stage` / `check` / `test` / `build` | exit 0; check 0 warnings; embedded 11 passed, 1240 filtered |
| `tools/package.sh` | `.hmod` 6,034 B `96d4e5c7…f769b367`, `.hnmod` 10,171 B `8bb3347c…9e27822c`; reproducible |
| `tools/e2e.sh` | `PASS` + `EXIT:0` |
| `SELFTEST=1 tools/e2e.sh` | `FAIL` + `EXIT:1` (fail-capability) |
| `check-e2e-logs.py buildinggadgets2 --strict` | 2/2 ok |

Evidence: `ports/_shared/evidence/buildinggadgets2/`.

## 2026-11-10 — content embedded in the `.hnmod` (AE2 pattern)

The port carried both an `.hmod` and an `.hnmod`, which the registry cannot list
(one file per project). The built content now travels **inside** the `.hnmod`:
`mixins/buildinggadgets2/data/buildinggadgets2-content.hmod` (byte-identical to
`content/buildinggadgets2.hmod`, `tools/package.py` refuses a stale copy) plus
`mixins/buildinggadgets2/embedded_content.rs`, whose `content_root` injection
writes it to `.horizon-cache/` and appends it to the dev-mod list.

| Check | Result |
|---|---|
| `cargo fmt --all -- --check`, `rustfmt --check …/embedded_content.rs` | exit 0 |
| `cargo clippy --all-targets --locked --offline -- -D warnings` | exit 0, 0 warnings |
| `cargo test --locked --offline` | 11 passed |
| `horizon.py stage` / `check` / `test` / `build` | exit 0; 0 warnings; embedded 11 passed, 1240 filtered |
| `tools/package.py` + `cargo horizon-mod check … --kind native-mod` | `.hnmod` 16,472 B `781f4e54…a3ed0c`, 11 entries incl. the data file; check exit 0 |
| `EMBEDDED=1 tools/e2e.sh` (no `--dev-mod`) | `PASS` + `EXIT:0`; control leg shows the gadget id unknown |
| `EMBEDDED=1 SELFTEST=1 tools/e2e.sh` | `FAIL` + `EXIT:1` |
| `check-e2e-logs.py buildinggadgets2 --strict` | 4/4 ok |

`upstream_ready.py buildinggadgets2` now exits **0** (`READY`), since the
one-file rule is met by the embedded bundle.

Evidence: `ports/_shared/evidence/buildinggadgets2/` (`ladder-embed.log`,
`e2e-embedded.log`, `e2e-embedded-selftest.log`, `check-native.log`,
`package-embed.log`).


## 2026-11-21 — slice 2, the mode family (0.2.3)

Pin **Horizon `0a875da91dbb0fdcb7ba8c42c11341aa229af51a`** + horizon-mixins
`e59da9094f0c2fdda6c627fe9eff22dec6a906f0`; `.horizon/sdk` deleted and refetched so the cached SDK
matches the new pin (the measured trap). Every step ran through `ports/tools/build-slot.sh`
(`BUILD_LOCAL=1`, `CARGO_INCREMENTAL=0`), logs in `../_shared/evidence/buildinggadgets2/`.

| Check | Result |
|---|---|
| `cargo test` | **30 passed**, 0 failed (`sdk-test.log`); the 11 new mode tests were red first (`sdk-test-red.log`, 19 passed / 11 failed, `EXIT:101`) |
| `cargo clippy --all-targets -- -D warnings` | exit 0, 0 warnings (`sdk-clippy.log`) |
| `cargo fmt --check` | exit 0 (`sdk-fmt.log`) |
| `cargo horizon-mod build` twice, `rm -f Cargo.lock` before each | **37,953 B**, sha256 `eed5258a64b413a462c28416fb14f17e64e4459f29bda97e5e7c44766d169920`, `cmp` identical (`sdk-build.log`, `sdk-build2.log`) |
| `cargo horizon-mod check buildinggadgets2.hmod` | `buildinggadgets2 0.2.3 (native-mod, format native-1/0a875da…/e59da90…, for 26.3)`, 37,801 bytes, exit 0 (`sdk-check.log`) |
| `cargo horizon-mod fits --build 0a875da` | `0a875da  made for`, exit 0 (`sdk-fits.log`) |
| `cargo horizon-mod fits --build 0b14f8a` | `0b14f8a  fits`, exit 0 (`sdk-fits-other.log`) |
| `verify-sdk-package.py buildinggadgets2` | 1/1 |
| `tools/e2e.sh` (`--offscreen`) | recipe + 9 mode phases: `PASS` + `EXIT:0` (`buildinggadgets2-e2e/e2e-modes.log`) |
| `GAME=$CONTROL_GAME tools/e2e.sh` | `FAIL: the carried gadget recipe did not craft…` + `EXIT:1` |
| `SELFTEST=1 tools/e2e.sh` | 9/9 phase assertions rejected the wrong expectation, `FAIL` + `EXIT:1` |
| `check-e2e-logs.py buildinggadgets2-e2e --strict` | logs=5 ok=5 |

The in-game leg asserts the exact placed cell set per mode (from the upstream Java, not the port's
planner), one stone consumed per cell, the parsed action-bar replies of `mode`/`range`/`placeontop`/
status, and the carried recipe's output and ingredient consumption with a missing-lapis control.

## 2026-11-23 — the held-cell preview slice: measured, parked, reverted

The next slice was the preview (`render_level_stage` + `draw_model_instances`). Its pure half was
green (39 tests; the four new ones red first at 36 passed / 3 failed, `EXIT:101`), and four
`--offscreen` diagnostic builds measured the client route:

| Measured | Result |
|---|---|
| `render_level_stage` at `AfterTranslucentBlocks` + `WorldDrawContext::quads` | fires and draws |
| `standalone_models` → `model_baking_completed` → `render_assets().model` | the package's own model id resolves |
| `draw_model_instances` with a tint | draws (green cube), tint and the vanilla-texture model apply |
| `local_player()` on the **render** hook's host | `None` — the plan cannot be made there |
| `player_inventory_slots` on the client | `None` (documented *Side: server*) |
| `render_hand`'s `HandRender::stack` components | `held=true`, but the gadget's state read back with **no selection** |

`blocked(no-client-held-stack-components)`: the pin has no client-side read of the main-hand stack
*with its item components*, which is what the preview needs. Separating "the hand copy drops the
component" from "the probe's out-of-reach sneak click was refused" needs another package build plus
an e2e slot; the wave's queue is the bottleneck, so the slice stops rather than shipping a preview
whose pixels were never observed. Evidence:
`../_shared/evidence/buildinggadgets2-preview-parked/` (two frames + the readings).

**Reverted to 0.2.4.** The slice's code, model asset, e2e phases and version bump were removed; the
rebuilt archive is **byte-identical** to the published one — `buildinggadgets2.hmod` **40,798 B**,
sha256 `927409d7750cc246e1e98630b6355bc9e90ba6c7cdf17c80f9e87bb05066f14a` — so **0.2.4 stays the
published version** and its evidence (`sdk-test.log` 35 passed, `e2e-hold*.log`) still describes the
delivered bytes.

Skills: superpowers/using-superpowers,brainstorming,writing-plans,test-driven-development,systematic-debugging,verification-before-completion; BeautifulRust/rust-best-practices
