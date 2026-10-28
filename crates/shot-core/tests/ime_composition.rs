use shot_core::ime::{ImeChange, ImeComposition};

#[test]
fn chinese_preedit_commits_once_without_leaking_composition() {
    let mut ime = ImeComposition::default();
    assert_eq!(
        ime.preedit("ni", 2, 0),
        ImeChange::Preedit {
            text: "ni".into(),
            active_range_chars: Some(2..2),
        }
    );
    assert!(ime.has_marked_text());
    assert_eq!(ime.marked_utf16_len(), 2);

    assert_eq!(
        ime.preedit("你", 1, 0),
        ImeChange::Preedit {
            text: "你".into(),
            active_range_chars: Some(1..1),
        }
    );
    assert_eq!(
        ime.commit("你"),
        ImeChange::Commit {
            text: "你".into(),
            was_marked: true,
        }
    );
    assert!(!ime.has_marked_text());
    assert_eq!(ime.cancel(), None, "no duplicate dismissal after commit");
}

#[test]
fn utf16_selection_converts_to_unicode_scalar_indices() {
    let mut ime = ImeComposition::default();
    assert_eq!(
        ime.preedit("A🙂中", 1, 2),
        ImeChange::Preedit {
            text: "A🙂中".into(),
            active_range_chars: Some(1..2),
        }
    );
    assert_eq!(ime.selected_range_utf16(), Some((1, 2)));
    assert_eq!(ime.marked_utf16_len(), 4);
    assert_eq!(
        ime.preedit("A🙂中", 2, 1),
        ImeChange::Preedit {
            text: "A🙂中".into(),
            active_range_chars: None,
        },
        "must not split UTF-16 surrogate pairs"
    );
}

#[test]
fn empty_preedit_and_focus_loss_clear_without_fabricating_commits() {
    let mut ime = ImeComposition::default();
    assert_eq!(ime.cancel(), None);
    ime.preedit("zhong", 5, 0);
    assert_eq!(
        ime.cancel(),
        Some(ImeChange::Preedit {
            text: String::new(),
            active_range_chars: None,
        })
    );
    assert!(!ime.has_marked_text());
    ime.preedit("文", 1, 0);
    assert_eq!(
        ime.preedit("", 0, 0),
        ImeChange::Preedit {
            text: String::new(),
            active_range_chars: None,
        }
    );
    assert_eq!(ime.cancel(), None);
}

#[test]
fn direct_text_without_marked_session_is_not_misclassified_as_ime() {
    let mut ime = ImeComposition::default();
    assert_eq!(
        ime.commit("test"),
        ImeChange::Commit {
            text: "test".into(),
            was_marked: false,
        }
    );
}

#[test]
fn cocoa_unmark_accepts_marked_text_without_a_duplicate_commit() {
    let mut ime = ImeComposition::default();
    ime.preedit("你好", 2, 0);
    assert_eq!(
        ime.unmark(),
        Some(ImeChange::Commit {
            text: "你好".into(),
            was_marked: true,
        })
    );
    assert!(!ime.has_marked_text());
    assert_eq!(ime.unmark(), None);
    assert_eq!(ime.cancel(), None);
}
