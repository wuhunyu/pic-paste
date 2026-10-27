# Bounded GPU texture residency for pinned images — Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Bound the number and estimated resident RGBA texture bytes of independent pinned images, avoid duplicate GPU uploads for identical images, and release all accounting and texture handles when the last pin closes.

**Architecture:** A dependency-free Rust `shot-core` ledger owns unique texture-group byte estimates, a per-group pin refcount, and an individual-pin map. The macOS app admits every new image *before* uploading to egui, reuses source + existing `egui::TextureHandle` only after exact dimensions and pixel-byte equality, and releases the ledger entry as closed pins are pruned. These checks are admission/retention estimates, **not measured GPU VRAM, RSS or frame timings**.

**Tech Stack:** Rust 2024, `shot-core`, egui 0.37 / eframe 0.37 with wgpu, Linux unit CI and macOS CI.

## Global constraints

- Screenshot, pinned-window, text/QR OCR, transparency and long-screenshot PNG-only behavior must remain unchanged.
- No Electron, WebView, network, dependency addition, or per-frame image reupload.
- Retain up to 12 live pins, but not more than 64 MiB of distinct resident RGBA textures; cap each pin image at 4,000,000 pixels and 4096 px per edge.
- Five distinct 1920×1080 RGBA images require exactly 41,472,000 bytes, below the 64 MiB budget.
- A duplicate image shares the source Arc and existing `TextureHandle`; only exact pixel equality permits sharing. Shared pins count toward the 12-pin limit but do not increase unique texture bytes.
- Dropping the final user of a texture releases its accounting immediately; actual wgpu reclamation is asynchronous.
- Reject oversized/exhausted requests **before** creating `egui::ColorImage` or `load_texture`, show a Chinese explanatory status, and never evict an existing visible pin silently.
- macOS 14+, multi-display/Spaces, five-1080p frame P95 ≤ 16.7 ms and CPU/RSS thresholds remain real-device acceptance in Issues #6/#10. CI success never claims those metrics.

---

## File ownership

- **Create** `crates/shot-core/src/pin_texture_budget.rs`: pure deterministic texture budget/ledger, errors and byte estimation; host independent.
- **Modify** `crates/shot-core/src/lib.rs`: export `pub mod pin_texture_budget`.
- **Create** `crates/shot-core/tests/pin_texture_budget.rs`: outside-in budget and reference lifetime tests, first committed as failing RED.
- **Modify** `apps/shot-macos/src/pin.rs`: `PinnedImage::from_shared` reuse of texture/source, `texture_group` id, deferred viewport close wakeup.
- **Modify** `apps/shot-macos/src/app.rs`: own `PinTextureBudget`, prune/release, exact image reuse, admission before `load_texture`, budget summary.
- **Modify** `README.md`, `docs/STATUS.md`, `docs/REMAINING.md`, `docs/TASKS.md` and Issue #6 for verified code-vs-device status.

### Task 1: Test the ledger contract (RED -> GREEN)

**Interface:**
```rust
pub const MAX_PIN_COUNT: usize = 12;
pub const MAX_TEXTURE_EDGE: u32 = 4096;
pub const MAX_TEXTURE_PIXELS: u64 = 4_000_000;
pub const MAX_TEXTURE_BYTES: u64 = 64 * 1024 * 1024;
pub enum PinBudgetError { InvalidDimensions, TextureTooLarge, TooManyPins, TextureBudgetExceeded, DuplicatePinId, MissingTextureGroup, SharedSizeMismatch }
pub struct PinTextureBudget { /* private ledgers */ }
impl PinTextureBudget {
    pub fn reserve(&mut self, pin_id: u64, width: u32, height: u32, reuse_group: Option<u64>) -> Result<u64, PinBudgetError>;
    pub fn release(&mut self, pin_id: u64) -> bool;
    pub fn active_pins(&self) -> usize;
    pub fn resident_bytes(&self) -> u64;
    pub fn validate_dimensions(width: u32, height: u32) -> Result<u64, PinBudgetError>;
}
```

- [x] Add failing `crates/shot-core/tests/pin_texture_budget.rs` tests using the desired API, covering 5×1080p success, aggregate limit, individual 4MP/4096px, 12-pin count, duplicate group references, releasing an owner before its shared copy, final release, repeated release, invalid size and unknown group.
- [x] Open draft PR #6; run Linux `cargo test -p shot-core --test pin_texture_budget --locked` in GitHub Actions and verify the expected missing module/API RED (not a non-feature infrastructure failure).
- [x] Implement `pin_texture_budget.rs` with checked u64 arithmetic, BTreeMaps from pin ID to group and group to `(bytes, refs)`, unique byte tracking. No silent eviction; `reserve` must not mutate state on any rejection.
- [x] Export module in `lib.rs`; run the tests GREEN, `cargo fmt --all -- --check`, `cargo clippy -p shot-core --all-targets --locked -- -D warnings`.

**Example user-facing behavior:** A ninth distinct 1080p pin is refused because 8 × 8,294,400 = 66,355,200 bytes; 9 × 8,294,400 = 74,649,600 bytes > 67,108,864 bytes. A second pin of the *same* 1080p image is admitted without extra GPU texture bytes.

### Task 2: macOS shared-texture lifecycle (RED -> GREEN)

**Interface:**
```rust
// apps/shot-macos/src/pin.rs
pub texture_group: u64,
pub fn from_shared(id: u64, other: &PinnedImage) -> PinnedImage;
pub fn new(ctx: &egui::Context, id: u64, frame: Arc<RgbaFrame>) -> PinnedImage;
pub enum PinEvent { /* preexisting variants */, Closed(u64) }
```

- [x] Write a macOS unit test in `pin.rs` constructing two pins from one `egui::Context` and checking equal TextureId and shared Arc, while both have independent closed state/mode.
- [x] Refactor the constructor through a private `with_texture` initializer. A new pin uploads once; `from_shared` clones the existing `TextureHandle` without calling `load_texture`; it does not copy the source pixels.
- [x] On pin viewport close or explicit ×, set `closed`, send `PinEvent::Closed(id)`, and wake ROOT. Add `Closed` handling in the root event dispatch.
- [x] Add `pin_budget: PinTextureBudget` in the root app. In `create_pin`, prune closed pins, reject unsafe dimensions/id/capacity, compare existing pins by exact frame dimensions and RGBA bytes, reserve group, then create/reuse pin. In `remove_closed_pins`, release matching ledger entries once and retain live pins; call from event loop and before admission.
- [x] Display `resident_bytes` as an **estimated** texture usage and live pin count in settings; never label it actual VRAM/RSS.
- [ ] Run workspace tests, fmt and strict Clippy on macOS CI and verify Lite/Full package builds; preserve PNG/OCR behavior.

### Task 3: Evidence and merge

- [x] Update README/status/remaining/tasks and Issue #6: resource cap is implemented, pin lifecycle unit evidence exists; keep real 5-pin P95, CPU/RSS, Spaces/Retina/clipboard validation **open**.
- [ ] Verify the latest PR branch’s full CI (Linux core, macOS workspace tests/Clippy/Release, OCR IPC, Lite/Full and Add-on packaging).
- [ ] Merge only after CI success and review, then sync the main branch ledger with PR link and leave Issue #6 open for physical-device acceptance.

## QA matrix

| Test | Expected |
|---|---|
| 5 different 1920×1080 pins | Admit; 41,472,000 estimated bytes |
| 8 different 1920×1080 pins | Admit; 66,355,200 estimated bytes |
| 9th different 1920×1080 pin | Reject, previous 8 unchanged |
| Same frame pinned twice | Same source Arc and TextureId; one allocation counted |
| Close original while duplicate remains | Texture stays resident |
| Close last duplicate | Byte accounting returns to 0, GPU handle dropped |
| 13th duplicate pin | Reject due to 12-pin count |
| 4097-px edge, >4MP, 0 dimension | Reject before GPU upload |
| 60,000-px long-image pin | Reject; original streaming PNG export unaffected |
| Mouse passthrough/drag/OCR | Existing behavior; close from hidden root wakes cleanup |
| Actual 5-pin P95 ≤16.7ms and CPU/RSS | Pending physical macOS measurements; never fabricate |

