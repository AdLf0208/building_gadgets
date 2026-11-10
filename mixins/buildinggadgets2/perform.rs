// Runs at the head of `Game::native_perform_command` (API 8): reads and stores
// the player's `/buildinggadgets2 range` selection and answers through
// `native_feedback`.
fn observe_command(
    game: &mut crate::game::Game<'_>,
    player: &NativeServerPlayer,
    command: &str,
    ran: &mut bool,
) {
    crate::game::buildinggadgets2_state::on_perform_command(game, player, command, ran);
}
