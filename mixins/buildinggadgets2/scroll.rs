// Runs at the head of `native_mouse_scrolling` (API 122): wheel steps over the
// held gadget change its range.
fn observe_wheel(
    game: &mut crate::game::Game<'_>,
    accumulated: [i32; 2],
    canceled: &mut bool,
) {
    crate::game::buildinggadgets2_state::on_mouse_scrolling(game, accumulated, canceled);
}
