pub mod dashboard;
pub mod footer;
pub mod gpu_view;
pub mod header;
pub mod help_view;
pub mod options_modal;
pub mod slots_view;
pub mod widgets;

use crate::app::{ActiveTab, App};
use dashboard::DashboardView;
use footer::Footer;
use gpu_view::GpuDetailsView;
use header::Header;
use help_view::HelpView;
use options_modal::OptionsModal;
use ratatui::{
    layout::{Constraint, Direction, Layout},
    Frame,
};
use slots_view::SlotsDetailsView;

/// Main draw function called on each terminal frame
pub fn draw(frame: &mut Frame, app: &App) {
    let area = frame.area();

    // If active theme specifies a solid background (or solid override), paint the canvas
    if let Some(bg_color) = app.theme.bg {
        frame.buffer_mut().set_style(area, ratatui::style::Style::default().bg(bg_color));
    }

    // Divide screen into Header (1 line), Body (fills remaining), Footer (1 line)
    let chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Length(1),
            Constraint::Min(10),
            Constraint::Length(1),
        ])
        .split(area);

    // 1. Render Top Header
    frame.render_widget(Header::new(app), chunks[0]);

    // 2. Render Main Body according to active tab
    match app.active_tab {
        ActiveTab::Dashboard => {
            frame.render_widget(DashboardView::new(app), chunks[1]);
        }
        ActiveTab::GpuDetails => {
            frame.render_widget(GpuDetailsView::new(app), chunks[1]);
        }
        ActiveTab::SlotsDetails => {
            frame.render_widget(SlotsDetailsView::new(app), chunks[1]);
        }
        ActiveTab::Help => {
            frame.render_widget(HelpView::new(&app.theme), chunks[1]);
        }
    }

    // 3. Render Bottom Footer
    frame.render_widget(Footer::new(&app.theme, app.is_paused), chunks[2]);

    // 4. If Options modal is open, render it centered on top of everything
    if app.options_menu_open {
        frame.render_widget(OptionsModal::new(app), area);
    }
}
