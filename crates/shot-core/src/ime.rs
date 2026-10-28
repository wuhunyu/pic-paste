//! Pure, platform-independent composition state for an NSTextInputClient bridge.
//!
//! AppKit positions and selections use UTF-16 code units. egui's preedit
//! active range uses Unicode scalar-value indices. A range cutting a
//! surrogate pair is never forwarded as a bogus character selection.

use std::ops::Range;

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum ImeChange {
    Preedit {
        text: String,
        active_range_chars: Option<Range<usize>>,
    },
    Commit {
        text: String,
        /// The native context already entered a marked-text session.
        /// Such commits must be sent as egui::ImeEvent::Commit, NOT Text.
        was_marked: bool,
    },
}

/// One local input-composition session. The surrounding document is owned
/// by egui; this type must not pretend to know its selection or content.
#[derive(Clone, Debug, Default)]
pub struct ImeComposition {
    marked: String,
    selected_utf16: Option<(usize, usize)>,
}

impl ImeComposition {
    pub fn has_marked_text(&self) -> bool {
        !self.marked.is_empty()
    }

    pub fn marked_utf16_len(&self) -> usize {
        self.marked.encode_utf16().count()
    }

    pub fn selected_range_utf16(&self) -> Option<(usize, usize)> {
        self.selected_utf16
    }

    pub fn marked_text(&self) -> &str {
        &self.marked
    }

    pub fn preedit(
        &mut self,
        text: impl Into<String>,
        selection_start_utf16: usize,
        selection_len_utf16: usize,
    ) -> ImeChange {
        let text = text.into();
        let active_range_chars = (!text.is_empty())
            .then_some(())
            .and_then(|()| selection_start_utf16.checked_add(selection_len_utf16))
            .and_then(|end| {
                Some((
                    char_offset_for_utf16(&text, selection_start_utf16)?,
                    char_offset_for_utf16(&text, end)?,
                ))
            })
            .map(|(start, end)| start..end);
        self.selected_utf16 = if text.is_empty() {
            None
        } else {
            active_range_chars
                .as_ref()
                .map(|_| (selection_start_utf16, selection_len_utf16))
        };
        self.marked = text.clone();
        ImeChange::Preedit {
            text,
            active_range_chars,
        }
    }

    pub fn commit(&mut self, text: impl Into<String>) -> ImeChange {
        let was_marked = self.has_marked_text();
        self.marked.clear();
        self.selected_utf16 = None;
        ImeChange::Commit {
            text: text.into(),
            was_marked,
        }
    }

    /// Dismiss the current preedit without submitting any text.
    /// Repeated cancellation is idempotent.
    pub fn cancel(&mut self) -> Option<ImeChange> {
        if !self.has_marked_text() {
            self.selected_utf16 = None;
            return None;
        }
        self.marked.clear();
        self.selected_utf16 = None;
        Some(ImeChange::Preedit {
            text: String::new(),
            active_range_chars: None,
        })
    }
}

fn char_offset_for_utf16(text: &str, offset: usize) -> Option<usize> {
    let mut utf16_offset = 0;
    let mut char_offset = 0;
    for ch in text.chars() {
        if utf16_offset == offset {
            return Some(char_offset);
        }
        utf16_offset += ch.len_utf16();
        char_offset += 1;
    }
    (utf16_offset == offset).then_some(char_offset)
}
