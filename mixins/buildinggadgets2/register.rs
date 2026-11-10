// Runs at the head of `Game::native_register_commands` (API 8): declares the
// `/buildinggadgets2 range` usage, the pin's replacement for the missing
// scroll-wheel range control.
fn declare_commands(commands: &mut Vec<NativeCommand>) {
    crate::game::buildinggadgets2_state::on_register_commands(commands);
}
