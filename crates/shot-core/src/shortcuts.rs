//! Versioned, validated keyboard shortcut preferences.
//! Modifier keys are intentionally fixed to Ctrl+Option in this increment.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ShortcutAction {
    Capture,
    PastePin,
    RecoverPins,
    FinishScroll,
}

impl ShortcutAction {
    pub const ALL: [Self; 4] = [
        Self::Capture,
        Self::PastePin,
        Self::RecoverPins,
        Self::FinishScroll,
    ];

    pub fn index(self) -> usize {
        match self {
            Self::Capture => 0,
            Self::PastePin => 1,
            Self::RecoverPins => 2,
            Self::FinishScroll => 3,
        }
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ShortcutError {
    InvalidDigit,
    DuplicateDigit,
    InvalidConfig,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct HotkeyBindings {
    digits: [u8; 4],
}

impl Default for HotkeyBindings {
    fn default() -> Self {
        Self {
            digits: [1, 2, 3, 4],
        }
    }
}

impl HotkeyBindings {
    pub fn new(digits: [u8; 4]) -> Result<Self, ShortcutError> {
        if digits.iter().any(|&v| !(1..=9).contains(&v)) {
            return Err(ShortcutError::InvalidDigit);
        }
        for (i, digit) in digits.iter().enumerate() {
            if digits[..i].contains(digit) {
                return Err(ShortcutError::DuplicateDigit);
            }
        }
        Ok(Self { digits })
    }

    pub fn digit(self, action: ShortcutAction) -> u8 {
        self.digits[action.index()]
    }

    pub fn with_digit(self, action: ShortcutAction, digit: u8) -> Result<Self, ShortcutError> {
        let mut digits = self.digits;
        digits[action.index()] = digit;
        Self::new(digits)
    }

    pub fn label(self, action: ShortcutAction) -> String {
        format!("Ctrl+Option+{}", self.digit(action))
    }

    pub fn serialize(self) -> String {
        format!(
            "version=1\ncapture={}\npaste={}\nrecover={}\nfinish={}\n",
            self.digits[0], self.digits[1], self.digits[2], self.digits[3]
        )
    }

    pub fn parse(value: &str) -> Result<Self, ShortcutError> {
        let mut lines = value.lines();
        if lines.next() != Some("version=1") {
            return Err(ShortcutError::InvalidConfig);
        }
        let mut digits = [0u8; 4];
        for (index, key) in ["capture", "paste", "recover", "finish"].iter().enumerate() {
            let Some(line) = lines.next() else {
                return Err(ShortcutError::InvalidConfig);
            };
            let Some((actual, number)) = line.split_once('=') else {
                return Err(ShortcutError::InvalidConfig);
            };
            if actual != *key || number.len() != 1 {
                return Err(ShortcutError::InvalidConfig);
            }
            digits[index] = number
                .parse::<u8>()
                .map_err(|_| ShortcutError::InvalidConfig)?;
        }
        if lines.next().is_some() {
            return Err(ShortcutError::InvalidConfig);
        }
        Self::new(digits)
    }
}
