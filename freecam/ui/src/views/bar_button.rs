use amml_ui::state::{DisabledReason, InteractionState};
use amml_ui::widget::{IconButton, TextButton};
use egui::{CursorIcon, Rect, Response, Sense, Ui};

use crate::id::FreecamUiId;

/// Puts amml-ui buttons in the fixed rects of the bar (`BarLayout`): each one takes clicks over its rect and paints
/// itself there. A button with a `DisabledReason` takes no clicks and shows the reason when hovered.
pub struct BarButton;

impl BarButton {
    pub fn icon(
        ui: &mut Ui,
        rect: Rect,
        id: FreecamUiId,
        button: &IconButton,
        disabled: Option<DisabledReason>,
    ) -> Response {
        let (response, state) = Self::interact(ui, rect, id, disabled.is_none());
        button.paint(ui.painter(), rect, state);
        Self::finish(response, disabled)
    }

    pub fn text(
        ui: &mut Ui,
        rect: Rect,
        id: FreecamUiId,
        button: &TextButton<'_>,
        disabled: Option<DisabledReason>,
    ) -> Response {
        let (response, state) = Self::interact(ui, rect, id, disabled.is_none());
        button.paint(ui.painter(), rect, state);
        Self::finish(response, disabled)
    }

    fn interact(ui: &mut Ui, rect: Rect, id: FreecamUiId, enabled: bool) -> (Response, InteractionState) {
        let sense = if enabled { Sense::click() } else { Sense::hover() };
        let response = ui.interact(rect, ui.scope_id().with(id), sense);
        let state = InteractionState::of(&response, enabled);
        (response, state)
    }

    fn finish(response: Response, disabled: Option<DisabledReason>) -> Response {
        match disabled {
            None => response.on_hover_cursor(CursorIcon::PointingHand),
            Some(reason) => reason.tooltip().attach(response),
        }
    }
}
