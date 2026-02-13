use crate::tui::app::{App, InputMode};
use crossterm::event::{KeyCode, KeyEvent, KeyModifiers};

/// Handle a key event, updating app state accordingly.
pub fn handle_key(app: &mut App, key: KeyEvent) {
    match app.mode {
        InputMode::Normal => handle_normal(app, key),
        InputMode::Searching => handle_search(app, key),
        InputMode::FilterMenu => handle_filter(app, key),
    }
}

fn handle_normal(app: &mut App, key: KeyEvent) {
    match key.code {
        // Quit
        KeyCode::Char('q') => app.should_quit = true,
        KeyCode::Char('c') if key.modifiers.contains(KeyModifiers::CONTROL) => {
            app.should_quit = true;
        }

        // Navigation
        KeyCode::Up | KeyCode::Char('k') => app.select_prev(),
        KeyCode::Down | KeyCode::Char('j') => app.select_next(),
        KeyCode::Home | KeyCode::Char('g') => app.selected = 0,
        KeyCode::End | KeyCode::Char('G') => {
            app.selected = app.visible_tasks.len().saturating_sub(1);
        }

        // Actions
        KeyCode::Char('c') => {
            let _ = app.complete_selected();
        }
        KeyCode::Char('d') => {
            let _ = app.delete_selected();
        }

        // Sort
        KeyCode::Char('s') => app.cycle_sort(),
        KeyCode::Char('S') => app.toggle_sort_reverse(),

        // Filter
        KeyCode::Char('f') => app.cycle_status_filter(),

        // Search
        KeyCode::Char('/') => {
            app.mode = InputMode::Searching;
            app.search_input.clear();
            app.status_msg = Some("Search: type query, Enter to apply, Esc to cancel".into());
        }

        // Clear search
        KeyCode::Esc => {
            app.clear_search();
            app.status_msg = None;
        }

        // Reload
        KeyCode::Char('r') => {
            let _ = app.load_tasks();
            app.status_msg = Some("Reloaded.".into());
        }

        _ => {}
    }
}

fn handle_search(app: &mut App, key: KeyEvent) {
    match key.code {
        KeyCode::Enter => {
            app.apply_search();
            app.mode = InputMode::Normal;
            app.status_msg = app
                .search_query
                .as_ref()
                .map(|q| format!("Search: \"{q}\""));
        }
        KeyCode::Esc => {
            app.mode = InputMode::Normal;
            app.search_input.clear();
            app.status_msg = None;
        }
        KeyCode::Backspace => {
            app.search_input.pop();
        }
        KeyCode::Char(c) => {
            app.search_input.push(c);
        }
        _ => {}
    }
}

fn handle_filter(app: &mut App, key: KeyEvent) {
    match key.code {
        KeyCode::Esc => {
            app.mode = InputMode::Normal;
        }
        _ => {
            app.mode = InputMode::Normal;
        }
    }
}
