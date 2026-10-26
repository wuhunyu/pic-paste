use shot_core::RgbaFrame;
use shot_scroll::live_preview::LatestPreview;

fn frame(alpha: u8) -> RgbaFrame {
    RgbaFrame::new(2, 2, vec![alpha; 16]).unwrap()
}

#[test]
fn pending_preview_is_taken_once_and_empty_afterwards() {
    let channel = LatestPreview::default();
    assert!(channel.take().is_none());
    channel.publish(frame(30));
    let next = channel.take().unwrap();
    assert_eq!(next.rgba[0], 30);
    assert!(channel.take().is_none());
}

#[test]
fn producer_overwrites_stale_frames_without_unbounded_queue() {
    let channel = LatestPreview::default();
    for i in 0..=250_u8 {
        channel.publish(frame(i));
    }
    assert_eq!(channel.take().unwrap().rgba[0], 250);
    assert!(channel.take().is_none());
}

#[test]
fn worker_and_ui_clones_share_latest_image() {
    let producer = LatestPreview::default();
    let consumer = producer.clone();
    producer.publish(frame(20));
    consumer.publish(frame(70));
    assert_eq!(producer.take().unwrap().rgba[0], 70);
    assert!(consumer.take().is_none());
}
