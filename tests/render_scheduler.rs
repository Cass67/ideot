use ideot::runtime::{RenderScheduler, TickAction};

#[test]
fn idle_ticks_do_not_request_repeated_redraws() {
    let mut scheduler = RenderScheduler::new();

    assert_eq!(scheduler.tick(), TickAction::Render);
    assert_eq!(scheduler.tick(), TickAction::WaitForInput);
    assert_eq!(scheduler.tick(), TickAction::WaitForInput);
}

#[test]
fn state_changes_request_one_redraw() {
    let mut scheduler = RenderScheduler::new();
    scheduler.tick();

    scheduler.mark_dirty();

    assert_eq!(scheduler.tick(), TickAction::Render);
    assert_eq!(scheduler.tick(), TickAction::WaitForInput);
}

#[test]
fn lsp_polling_interval_is_independent_from_rendering() {
    let mut scheduler = RenderScheduler::new();
    scheduler.tick();

    assert_eq!(scheduler.tick(), TickAction::WaitForInput);
    assert!(scheduler.should_poll_lsp_after_idle());
}
