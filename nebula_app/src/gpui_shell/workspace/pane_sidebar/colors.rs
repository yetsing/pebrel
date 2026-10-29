//! Sidebar foregrounds; surfaces and interaction backgrounds belong to the theme.

use gpui::{App, Hsla, rgb};
use gpui_component::ActiveTheme as _;

pub(super) struct Colors {
    pub(super) project: Hsla,
    pub(super) foreground: Hsla,
    pub(super) muted: Hsla,
    pub(super) icon: Hsla,
    pub(super) tree: Hsla,
    pub(super) pin: Hsla,
    pub(super) branch: Hsla,
    pub(super) selection_indicator: Hsla,
    pub(super) ready: Hsla,
    pub(super) shell: Hsla,
}

impl Colors {
    pub(super) fn new(cx: &App) -> Self {
        let theme = cx.theme();
        if theme.is_dark() {
            Self {
                project: rgb(0xb4d6a7).into(),
                foreground: rgb(0xd1d3e0).into(),
                muted: rgb(0x9c9db0).into(),
                icon: rgb(0x8f93a8).into(),
                tree: rgb(0x55586c).into(),
                pin: rgb(0x8ca9ed).into(),
                branch: rgb(0xd1a0b7).into(),
                selection_indicator: rgb(0xb0b5e2).into(),
                ready: rgb(0x9bc37b).into(),
                shell: rgb(0x8c9fce).into(),
            }
        } else {
            Self {
                project: rgb(0x476c40).into(),
                foreground: theme.foreground,
                muted: theme.muted_foreground,
                icon: rgb(0x60667b).into(),
                tree: rgb(0xbabdca).into(),
                pin: rgb(0x4563aa).into(),
                branch: rgb(0xa34e78).into(),
                selection_indicator: rgb(0x6b70b5).into(),
                ready: rgb(0x527b37).into(),
                shell: rgb(0x586da1).into(),
            }
        }
    }
}
