//! Building Gadgets 2 for Horizon — the building gadget and its eight modes.
//!
//! Building Gadgets 2 (see `FINDINGS.md`) gives one item, the building gadget: a
//! sneak right click stores the clicked block in the gadget, a plain right click then
//! builds that block's selected mode, one item of the block's kind per placed cell.
//!
//! The gadget's own settings live in its item component
//! (`buildinggadgets2:gadget_state`), exactly where upstream keeps them, so they
//! travel with the stack: the range, the selection, the mode and the
//! `place_on_top` toggle.
//!
//! [`runtime`] is the pure planner and command parser (one collector per upstream
//! mode, unit-tested on its own); [`support`] is the glue that reads the world, the
//! inventory and the component, and writes the blocks. This file is the one
//! `impl Hooks`: the item part that answers a right click, the declared component,
//! the commands and the mouse wheel.

pub mod runtime;
pub mod support;

use mod_api::{
    self, ClientHost, ComponentTypesEvent, Flow, Hooks, MouseScrollingEvent, RegisterCommandsEvent,
    Registrar, RegistrationContext, RunModCommandEvent, ServerHost,
};

/// The mod has no state of its own: the gadget's state is in the stack's component,
/// and the builder derives everything else from the click.
#[derive(Default)]
struct Mod;

#[mod_api::horizon_mod]
impl Hooks for Mod {
    /// Attach the gadget's behaviour to the item the package's content declares. A
    /// Part is scoped: the game asks it only for `buildinggadgets2:gadget_building`,
    /// so the hook never has to check the item.
    fn register(&self, registrar: &mut dyn Registrar, _context: &RegistrationContext) {
        let gadget = registrar.item_key(runtime::GADGET);
        registrar.item_part(gadget, support::Gadget);
        // Slice 0.2.7: the destruction gadget plans a destination on a sneak click. Its
        // part is scoped to that item, so the two gadgets cannot answer each other's clicks.
        let destroy = registrar.item_key(runtime::DESTROY_GADGET);
        registrar.item_part(destroy, support::DestroyGadget);
        // Slice 0.2.8: the exchange gadget plans a destination on a sneak click with the
        // same three measured queries; its part is scoped to the jar's `gadget_exchanging`.
        let exchange = registrar.item_key(runtime::EXCHANGE_GADGET);
        registrar.item_part(exchange, support::ExchangeGadget);
    }

    /// Declare the component that holds the gadget's range and selection, before
    /// content (and so the item's stack codec) loads.
    fn component_types(&self, _context: &RegistrationContext, event: &mut ComponentTypesEvent<'_>) {
        support::declare_component(event.types);
    }

    /// `/buildinggadgets2 [range [n]]`: the range control the wheel steps and the
    /// chat path share.
    fn register_commands(&self, _host: &mut dyn ServerHost, event: &mut RegisterCommandsEvent<'_>) {
        support::declare_commands(event.commands);
    }

    /// A player ran one of the commands above: answer it and take it.
    fn run_mod_command(
        &self,
        host: &mut dyn ServerHost,
        event: &mut RunModCommandEvent<'_>,
    ) -> Flow {
        support::run_command(host, event)
    }

    /// A wheel notch in gameplay: step the held gadget's range and send the same
    /// server command the chat path uses, since only the server writes the stack.
    fn mouse_scrolling(&self, host: &mut dyn ClientHost, event: &mut MouseScrollingEvent) -> Flow {
        support::scroll(host, event)
    }
}
