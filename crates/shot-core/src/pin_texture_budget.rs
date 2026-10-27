//! Deterministic GPU texture residency accounting for macOS pinned images.
//!
//! Bytes are estimated RGBA8 texture payload, not actual VRAM/RSS or wgpu
//! allocation sizes. No existing visible pin is evicted to satisfy a request.
use std::collections::BTreeMap;
use std::fmt;

pub const MAX_PIN_COUNT: usize = 12;
pub const MAX_TEXTURE_EDGE: u32 = 4096;
pub const MAX_TEXTURE_PIXELS: u64 = 4_000_000;
pub const MAX_TEXTURE_BYTES: u64 = 64 * 1024 * 1024;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum PinBudgetError {
    InvalidDimensions,
    TextureTooLarge,
    TooManyPins,
    TextureBudgetExceeded,
    DuplicatePinId,
    MissingTextureGroup,
    SharedSizeMismatch,
}

impl fmt::Display for PinBudgetError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::InvalidDimensions => write!(f, "贴图宽高必须大于 0"),
            Self::TextureTooLarge => {
                write!(f, "单张贴图最多 400 万像素、单边不超过 4096 px")
            }
            Self::TooManyPins => write!(f, "最多同时打开 12 张悬浮贴图，请先关闭一张"),
            Self::TextureBudgetExceeded => {
                write!(f, "贴图 GPU 纹理估算预算已满（64 MiB），请先关闭部分贴图")
            }
            Self::DuplicatePinId => write!(f, "贴图编号冲突，请重新创建"),
            Self::MissingTextureGroup => write!(f, "待复用的贴图纹理已不存在"),
            Self::SharedSizeMismatch => write!(f, "贴图尺寸与复用的纹理不一致"),
        }
    }
}

impl std::error::Error for PinBudgetError {}

#[derive(Clone, Copy, Debug)]
struct TextureAllocation {
    width: u32,
    height: u32,
    rgba_bytes: u64,
    references: usize,
}

/// Reference-counted accounting independent of egui and the OS GPU backend.
///
/// Group IDs are the pin ID that first created a texture. They outlive that
/// original pin while other pins still reference the same texture.
#[derive(Debug, Default)]
pub struct PinTextureBudget {
    pin_groups: BTreeMap<u64, u64>,
    textures: BTreeMap<u64, TextureAllocation>,
    resident_bytes: u64,
}

impl PinTextureBudget {
    pub fn active_pins(&self) -> usize {
        self.pin_groups.len()
    }

    pub fn resident_bytes(&self) -> u64 {
        self.resident_bytes
    }

    /// Validate *before* ColorImage conversion or GPU allocation.
    pub fn validate_dimensions(width: u32, height: u32) -> Result<u64, PinBudgetError> {
        if width == 0 || height == 0 {
            return Err(PinBudgetError::InvalidDimensions);
        }
        if width > MAX_TEXTURE_EDGE || height > MAX_TEXTURE_EDGE {
            return Err(PinBudgetError::TextureTooLarge);
        }
        let pixels = u64::from(width)
            .checked_mul(u64::from(height))
            .ok_or(PinBudgetError::TextureTooLarge)?;
        if pixels > MAX_TEXTURE_PIXELS {
            return Err(PinBudgetError::TextureTooLarge);
        }
        pixels.checked_mul(4).ok_or(PinBudgetError::TextureTooLarge)
    }

    /// Reserve one pin ID. An existing texture group may be reused ONLY when
    /// the caller verified identical dimensions and every RGBA source byte.
    /// Every rejection leaves the ledger unchanged.
    pub fn reserve(
        &mut self,
        pin_id: u64,
        width: u32,
        height: u32,
        reuse_group: Option<u64>,
    ) -> Result<u64, PinBudgetError> {
        let bytes = Self::validate_dimensions(width, height)?;
        if self.pin_groups.contains_key(&pin_id) || self.textures.contains_key(&pin_id) {
            return Err(PinBudgetError::DuplicatePinId);
        }
        if self.active_pins() >= MAX_PIN_COUNT {
            return Err(PinBudgetError::TooManyPins);
        }

        let group = if let Some(group) = reuse_group {
            let allocation = self
                .textures
                .get(&group)
                .ok_or(PinBudgetError::MissingTextureGroup)?;
            if allocation.width != width
                || allocation.height != height
                || allocation.rgba_bytes != bytes
            {
                return Err(PinBudgetError::SharedSizeMismatch);
            }
            group
        } else {
            self.resident_bytes
                .checked_add(bytes)
                .filter(|total| *total <= MAX_TEXTURE_BYTES)
                .ok_or(PinBudgetError::TextureBudgetExceeded)?;
            pin_id
        };

        if let Some(existing) = self.textures.get_mut(&group) {
            existing.references += 1;
        } else {
            self.textures.insert(
                group,
                TextureAllocation {
                    width,
                    height,
                    rgba_bytes: bytes,
                    references: 1,
                },
            );
            self.resident_bytes += bytes;
        }
        self.pin_groups.insert(pin_id, group);
        Ok(group)
    }

    /// Idempotent close. A shared texture stays accounted until its last user
    /// closes; actual wgpu resource reclamation occurs asynchronously.
    pub fn release(&mut self, pin_id: u64) -> bool {
        let Some(group) = self.pin_groups.remove(&pin_id) else {
            return false;
        };
        let last_reference = {
            let allocation = self
                .textures
                .get_mut(&group)
                .expect("every pin must reference a resident texture group");
            allocation.references -= 1;
            allocation.references == 0
        };
        if last_reference {
            let allocation = self
                .textures
                .remove(&group)
                .expect("last texture group must exist");
            self.resident_bytes -= allocation.rgba_bytes;
        }
        true
    }
}
