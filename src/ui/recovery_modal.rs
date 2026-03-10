use ratatui::Frame;
use ratatui::style::{Modifier, Style};

use crate::state::{HealingStep, RecoveryModalState, RecoveryStage};
use crate::ui::popup;
use crate::ui::theme;

const POPUP_WIDTH: u16 = 56;
const POPUP_HEIGHT: u16 = 14;

pub fn render(state: &RecoveryModalState, frame: &mut Frame, phase: f32) {
    let area = frame.area();
    if area.width < 30 || area.height < 10 {
        return;
    }

    let popup_w = POPUP_WIDTH.min(area.width.saturating_sub(4));
    let popup_h = POPUP_HEIGHT.min(area.height.saturating_sub(4));
    let popup_area = popup::centered_rect(popup_w, popup_h, area);
    let buf = frame.buffer_mut();
    let bounds = *buf.area();

    popup::render_popup_chrome(buf, &bounds, popup_area, "RECOVERY", phase);

    let left = popup_area.x + 3;
    let inner_w = (popup_area.width.saturating_sub(6)) as usize;
    let mut y = popup_area.y + 3;

    let text_style = Style::default().fg(theme::TEXT).bg(theme::BG);
    let dim_style = Style::default().fg(theme::DIM).bg(theme::BG);
    let key_style = Style::default()
        .fg(theme::MAGENTA)
        .bg(theme::BG)
        .add_modifier(Modifier::BOLD);
    let green_style = Style::default().fg(theme::GREEN).bg(theme::BG);
    let red_style = Style::default().fg(theme::RED).bg(theme::BG);

    match &state.stage {
        RecoveryStage::Checking => {
            popup::write_str(
                buf,
                &bounds,
                left,
                y,
                "Checking recovery status...",
                dim_style,
            );
        }
        RecoveryStage::Disabled => {
            popup::write_str(buf, &bounds, left, y, "No recovery key found", text_style);
            y += 2;
            popup::write_str(buf, &bounds, left, y, "[Enter]", key_style);
            popup::write_str(buf, &bounds, left + 8, y, "Create recovery key", text_style);
        }
        RecoveryStage::Enabled => {
            popup::write_str(buf, &bounds, left, y, "Recovery is enabled", green_style);
            y += 2;
            popup::write_str(buf, &bounds, left, y, "[r]", key_style);
            popup::write_str(buf, &bounds, left + 4, y, "Reset key", text_style);
        }
        RecoveryStage::Incomplete => {
            popup::write_str(
                buf,
                &bounds,
                left,
                y,
                "Recovery exists but not cached locally",
                text_style,
            );
            y += 2;
            popup::write_str(buf, &bounds, left, y, "[e]", key_style);
            popup::write_str(buf, &bounds, left + 4, y, "Enter key", text_style);
            popup::write_str(buf, &bounds, left + 16, y, "[r]", key_style);
            popup::write_str(buf, &bounds, left + 20, y, "Reset", text_style);
        }
        RecoveryStage::EnterKey => {
            popup::write_str(buf, &bounds, left, y, "Enter recovery key:", text_style);
            y += 2;
            let display = if state.key_buffer.is_empty() {
                "█".to_string()
            } else {
                let visible: String = state
                    .key_buffer
                    .chars()
                    .take(inner_w.saturating_sub(1))
                    .collect();
                format!("{visible}█")
            };
            popup::write_str(buf, &bounds, left, y, &display, green_style);
            y += 2;
            popup::write_str(buf, &bounds, left, y, "[Enter]", key_style);
            popup::write_str(buf, &bounds, left + 8, y, "Submit", dim_style);
        }
        RecoveryStage::NeedPassword => {
            popup::write_str(buf, &bounds, left, y, "Password required for", text_style);
            y += 1;
            popup::write_str(buf, &bounds, left, y, "cross-signing reset:", text_style);
            y += 2;
            let masked: String =
                "•".repeat(state.password_buffer.len().min(inner_w.saturating_sub(1)));
            let display = format!("{masked}█");
            popup::write_str(buf, &bounds, left, y, &display, green_style);
            y += 2;
            popup::write_str(buf, &bounds, left, y, "[Enter]", key_style);
            popup::write_str(buf, &bounds, left + 8, y, "Submit", dim_style);
        }
        RecoveryStage::Healing(step) => {
            let msg = match step {
                HealingStep::CrossSigning => "Setting up cross-signing...",
                HealingStep::Backup => "Enabling backup...",
                HealingStep::ExportSecrets => "Exporting secrets to new recovery key...",
            };
            popup::write_str(buf, &bounds, left, y, msg, dim_style);
        }
        RecoveryStage::Recovering => {
            popup::write_str(
                buf,
                &bounds,
                left,
                y,
                "Recovering from phrase...",
                dim_style,
            );
        }
        RecoveryStage::Creating => {
            popup::write_str(buf, &bounds, left, y, "Creating recovery key...", dim_style);
        }
        RecoveryStage::ShowKey(key) => {
            popup::write_str(buf, &bounds, left, y, "Save this recovery key:", text_style);
            y += 2;
            let truncated: String = key.chars().take(inner_w).collect();
            popup::write_str(buf, &bounds, left, y, &truncated, green_style);
            y += 2;
            popup::write_str(buf, &bounds, left, y, "[c]", key_style);
            let copy_label = if state.copied { "Copied!" } else { "Copy" };
            popup::write_str(buf, &bounds, left + 4, y, copy_label, text_style);
            popup::write_str(buf, &bounds, left + 16, y, "[Enter]", key_style);
            popup::write_str(buf, &bounds, left + 24, y, "Done", text_style);
        }
        RecoveryStage::ConfirmReset => {
            popup::write_str(
                buf,
                &bounds,
                left,
                y,
                "Type 'yes' to confirm reset:",
                red_style,
            );
            y += 2;
            let display = if state.confirm_buffer.is_empty() {
                "█".to_string()
            } else {
                let visible: String = state
                    .confirm_buffer
                    .chars()
                    .take(inner_w.saturating_sub(1))
                    .collect();
                format!("{visible}█")
            };
            popup::write_str(buf, &bounds, left, y, &display, text_style);
        }
        RecoveryStage::Resetting => {
            popup::write_str(
                buf,
                &bounds,
                left,
                y,
                "Resetting recovery key...",
                dim_style,
            );
        }
        RecoveryStage::Error(msg) => {
            popup::write_str(buf, &bounds, left, y, msg, red_style);
            y += 2;
            popup::write_str(buf, &bounds, left, y, "[Enter]", key_style);
            popup::write_str(buf, &bounds, left + 8, y, "Close", dim_style);
        }
    }

    // Hint at bottom
    popup::render_hint(buf, &bounds, popup_area, "Esc close");
}
