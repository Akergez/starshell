use gpui::{ColorExt, IntoElement};
use ui::components::{Input, InputState};
use ui::theme::Theme;

pub fn render_auth_form(
    theme: &Theme,
    password_input: &InputState,
    auth_failed: bool,
    is_checking: bool,
) -> impl IntoElement {
    let (enter_pwd, checking_text, incorrect_text) =
        ("Password", "Checking…", "Incorrect password");

    let placeholder = if is_checking {
        checking_text
    } else if auth_failed && password_input.is_empty() {
        incorrect_text
    } else {
        enter_pwd
    };

    Input::from_state("lockscreen-password-input", password_input)
        .placeholder(placeholder)
        .masked(true)
        .focused(true)
        .cursor_visible(false)
        .error(auth_failed)
        .centered(true)
        .w_px(200.0)
        .max_w_px(200.0)
        .h_px(34.0)
        .rounded_full()
        .bg(theme.elevated().opacity(0.35))
        .border_color(if auth_failed {
            theme.error()
        } else {
            theme.elevated().opacity(0.35)
        })
        .focused_border_color(if auth_failed {
            theme.error()
        } else {
            theme.accent()
        })
}
