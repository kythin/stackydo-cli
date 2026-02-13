use crate::model::task::Priority;
use crate::tui::app::{App, CreateField, InputMode, SettingsField};
use crossterm::event::{KeyCode, KeyEvent, KeyModifiers};

/// Handle a key event, updating app state accordingly.
pub fn handle_key(app: &mut App, key: KeyEvent) {
    match &app.mode {
        InputMode::Normal => handle_normal(app, key),
        InputMode::Searching => handle_search(app, key),
        InputMode::FilterMenu => handle_filter(app, key),
        InputMode::Creating(_) => handle_create(app, key),
        InputMode::Settings => handle_settings(app, key),
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

        // Create task
        KeyCode::Char('n') => {
            app.create_state = Default::default();
            app.mode = InputMode::Creating(CreateField::Title);
            app.status_msg = Some("New task: Tab=next field, Enter=create, Esc=cancel".into());
        }

        // Settings
        KeyCode::Char(',') => {
            app.mode = InputMode::Settings;
            app.settings_field = SettingsField::DefaultSort;
            app.status_msg = Some("Settings: j/k=navigate, Enter/Space=toggle, s=save, Esc=back".into());
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

fn handle_create(app: &mut App, key: KeyEvent) {
    let field = match &app.mode {
        InputMode::Creating(f) => f.clone(),
        _ => return,
    };

    match key.code {
        KeyCode::Esc => {
            app.create_state = Default::default();
            app.mode = InputMode::Normal;
            app.status_msg = Some("Create cancelled.".into());
        }
        KeyCode::Tab | KeyCode::BackTab => {
            let next = if key.code == KeyCode::BackTab {
                field.prev()
            } else {
                field.next()
            };
            app.mode = InputMode::Creating(next);
        }
        KeyCode::Enter => {
            // Submit the form
            let _ = app.submit_create();
            app.mode = InputMode::Normal;
        }
        KeyCode::Backspace => {
            match field {
                CreateField::Title => { app.create_state.title.pop(); }
                CreateField::Tags => { app.create_state.tags.pop(); }
                CreateField::Stack => { app.create_state.stack.pop(); }
                CreateField::Body => { app.create_state.body.pop(); }
                CreateField::Priority => {
                    // Clear priority
                    app.create_state.priority = None;
                }
            }
        }
        KeyCode::Char(c) => {
            match field {
                CreateField::Title => app.create_state.title.push(c),
                CreateField::Tags => app.create_state.tags.push(c),
                CreateField::Stack => app.create_state.stack.push(c),
                CreateField::Body => app.create_state.body.push(c),
                CreateField::Priority => {
                    // Cycle through priorities on any char input
                    app.create_state.priority = match &app.create_state.priority {
                        None => Some(Priority::Low),
                        Some(Priority::Low) => Some(Priority::Medium),
                        Some(Priority::Medium) => Some(Priority::High),
                        Some(Priority::High) => Some(Priority::Critical),
                        Some(Priority::Critical) => None,
                    };
                }
            }
        }
        _ => {}
    }
}

fn handle_settings(app: &mut App, key: KeyEvent) {
    match key.code {
        KeyCode::Esc => {
            app.mode = InputMode::Normal;
            app.status_msg = None;
        }
        KeyCode::Up | KeyCode::Char('k') => {
            app.settings_field = app.settings_field.prev();
        }
        KeyCode::Down | KeyCode::Char('j') => {
            app.settings_field = app.settings_field.next();
        }
        KeyCode::Enter | KeyCode::Char(' ') => {
            // Toggle/cycle the selected setting
            match app.settings_field {
                SettingsField::DefaultSort => {
                    app.settings.default_sort = match app.settings.default_sort.as_str() {
                        "created" => "due".into(),
                        "due" => "priority".into(),
                        "priority" => "modified".into(),
                        _ => "created".into(),
                    };
                }
                SettingsField::DefaultFilter => {
                    app.settings.default_filter_status = match &app.settings.default_filter_status {
                        None => Some("todo".into()),
                        Some(s) if s == "todo" => Some("in_progress".into()),
                        Some(s) if s == "in_progress" => Some("done".into()),
                        Some(s) if s == "done" => Some("blocked".into()),
                        Some(s) if s == "blocked" => Some("cancelled".into()),
                        _ => None,
                    };
                }
                SettingsField::AutoCaptureGit => {
                    app.settings.auto_capture_git = !app.settings.auto_capture_git;
                }
                SettingsField::QuickListLimit => {
                    app.settings.quick_list_limit = match app.settings.quick_list_limit {
                        10 => 25,
                        25 => 50,
                        50 => 100,
                        100 => 200,
                        _ => 10,
                    };
                }
            }
        }
        KeyCode::Char('s') => {
            let _ = app.save_settings();
        }
        _ => {}
    }
}
