use shot_core::async_ticket::OcrTicket;
use shot_core::RectPx;

fn box_at(x: i32) -> RectPx {
    RectPx {
        x,
        y: 3,
        width: 40,
        height: 20,
    }
}

#[test]
fn old_ocr_results_must_not_bind_after_image_edit() {
    let ticket = OcrTicket::new(8, 4, box_at(12));
    assert!(ticket.matches(8, 4, box_at(12)));
    assert!(!ticket.matches(8, 5, box_at(12)));
}

#[test]
fn old_ocr_results_must_not_bind_after_new_capture_or_selection() {
    let ticket = OcrTicket::new(8, 4, box_at(12));
    assert!(!ticket.matches(9, 4, box_at(12)));
    assert!(!ticket.matches(8, 4, box_at(13)));
    let different_size = RectPx {
        x: 12,
        y: 3,
        width: 41,
        height: 20,
    };
    assert!(!ticket.matches(8, 4, different_size));
}
