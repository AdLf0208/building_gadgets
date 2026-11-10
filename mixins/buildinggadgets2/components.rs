// Runs at the head of `native_component_types` (API 65): declares the gadget's
// persisted state component before content loads.
fn declare_components(types: &mut Vec<String>) {
    crate::game::buildinggadgets2_state::on_component_types(types);
}
