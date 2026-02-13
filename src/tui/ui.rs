use crate::model::task::Priority;
use crate::tui::app::{App, CreateField, InputMode, SettingsField};
use crate::tui::widgets::task_list::format_task_line;
use ratatui::{
    layout::{Constraint, Direction, Layout, Rect},
    style::{Color, Modifier, Style},
    text::{Line, Span},
    widgets::{Block, Borders, List, Paragraph, Wrap},
    Frame,
};

/// Render the full TUI layout.
pub fn draw(f: &mut Frame, app: &App) {
    match &app.mode {
        InputMode::Creating(_) => draw_create_screen(f, app),
        InputMode::Settings => draw_settings_screen(f, app),
        _ => draw_main_screen(f, app),
    }
}

fn draw_main_screen(f: &mut Frame, app: &App) {
    let chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Length(3), // Context bar
            Constraint::Min(3),   // Main area
            Constraint::Length(1), // Status bar
        ])
        .split(f.size());

    draw_context_bar(f, app, chunks[0]);

    let main = Layout::default()
        .direction(Direction::Horizontal)
        .constraints([Constraint::Percentage(40), Constraint::Percentage(60)])
        .split(chunks[1]);

    draw_task_list(f, app, main[0]);
    draw_detail_pane(f, app, main[1]);
    draw_status_bar(f, app, chunks[2]);
}

fn draw_context_bar(f: &mut Frame, app: &App, area: Rect) {
    let mut spans = vec![Span::styled(" ctx ", Style::default().fg(Color::Black).bg(Color::Cyan))];

    if let Some(ref ctx) = app.context {
        let c = &ctx.context;

        spans.push(Span::raw(format!(" {} ", c.working_dir)));

        if let Some(ref branch) = c.git_branch {
            spans.push(Span::styled(
                format!(" {branch} "),
                Style::default().fg(Color::Green),
            ));
            if let Some(ref commit) = c.git_commit {
                spans.push(Span::styled(
                    format!("@{commit} "),
                    Style::default().fg(Color::DarkGray),
                ));
            }
        }

        if let Some(ref cfg) = ctx.config_file_path {
            spans.push(Span::styled(
                format!(" cfg:{cfg}"),
                Style::default().fg(Color::Yellow),
            ));
        }
    } else {
        spans.push(Span::raw(" (no context) "));
    }

    let bar = Paragraph::new(Line::from(spans))
        .block(Block::default().borders(Borders::BOTTOM));

    f.render_widget(bar, area);
}

fn draw_task_list(f: &mut Frame, app: &App, area: Rect) {
    let items: Vec<_> = app
        .visible_tasks
        .iter()
        .enumerate()
        .map(|(view_idx, &task_idx)| {
            let task = &app.all_tasks[task_idx];
            format_task_line(task, view_idx == app.selected)
        })
        .collect();

    let filter_str = app
        .filters
        .status
        .as_ref()
        .map(|s| format!(" [{s}]"))
        .unwrap_or_default();

    let title = format!(
        " Tasks ({}){} — sort:{} ",
        app.visible_tasks.len(),
        filter_str,
        app.sort_field.label(),
    );

    let list = List::new(items).block(Block::default().title(title).borders(Borders::ALL));

    f.render_widget(list, area);
}

fn draw_detail_pane(f: &mut Frame, app: &App, area: Rect) {
    let content = if let Some(task) = app.selected_task() {
        let fm = &task.frontmatter;
        let mut lines = vec![
            Line::from(Span::styled(
                fm.title.clone(),
                Style::default()
                    .fg(Color::Cyan)
                    .add_modifier(Modifier::BOLD),
            )),
            Line::from(""),
            Line::from(format!("Status:   {}", fm.status)),
        ];

        if let Some(ref p) = fm.priority {
            let color = match p {
                Priority::Critical => Color::Red,
                Priority::High => Color::Yellow,
                Priority::Medium => Color::Blue,
                Priority::Low => Color::Gray,
            };
            lines.push(Line::from(Span::styled(
                format!("Priority: {p}"),
                Style::default().fg(color),
            )));
        }

        if let Some(due) = fm.due {
            lines.push(Line::from(format!(
                "Due:      {}",
                due.format("%Y-%m-%d %H:%M")
            )));
        }

        if !fm.tags.is_empty() {
            lines.push(Line::from(format!("Tags:     {}", fm.tags.join(", "))));
        }

        if let Some(ref stack) = fm.stack {
            lines.push(Line::from(format!("Stack:    {stack}")));
        }

        lines.push(Line::from(format!(
            "Created:  {}",
            fm.created.format("%Y-%m-%d %H:%M")
        )));
        lines.push(Line::from(format!("ID:       {}", fm.id)));

        // Context section
        lines.push(Line::from(""));
        lines.push(Line::from(Span::styled(
            "Context",
            Style::default().add_modifier(Modifier::UNDERLINED),
        )));
        lines.push(Line::from(format!("Dir: {}", fm.context.working_dir)));
        if let Some(ref branch) = fm.context.git_branch {
            lines.push(Line::from(format!("Git: {branch}")));
        }
        if let Some(ref path) = fm.context.path {
            let mut ctx_line = format!("Path: {path}");
            if let Some(line) = fm.context.line {
                ctx_line.push_str(&format!(":{line}"));
            }
            lines.push(Line::from(ctx_line));
        }
        if let Some(ref lf) = fm.context.lookfor {
            lines.push(Line::from(format!("Lookfor: {lf}")));
        }

        // Dependencies
        if !fm.dependencies.is_empty() {
            lines.push(Line::from(""));
            lines.push(Line::from(Span::styled(
                "Dependencies",
                Style::default().add_modifier(Modifier::UNDERLINED),
            )));
            for dep in &fm.dependencies {
                let id_short = if dep.task_id.len() > 10 {
                    &dep.task_id[..10]
                } else {
                    &dep.task_id
                };
                lines.push(Line::from(format!("  {:?} → {id_short}", dep.dep_type)));
            }
        }

        // Body
        if !task.body.is_empty() {
            lines.push(Line::from(""));
            lines.push(Line::from(Span::styled(
                "Body",
                Style::default().add_modifier(Modifier::UNDERLINED),
            )));
            for body_line in task.body.lines() {
                lines.push(Line::from(body_line.to_string()));
            }
        }

        lines
    } else {
        vec![Line::from("No task selected.")]
    };

    let detail = Paragraph::new(content)
        .block(Block::default().title(" Detail ").borders(Borders::ALL))
        .wrap(Wrap { trim: false });

    f.render_widget(detail, area);
}

fn draw_status_bar(f: &mut Frame, app: &App, area: Rect) {
    let msg = match &app.mode {
        InputMode::Searching => format!("/{}", app.search_input),
        _ => app.status_msg.clone().unwrap_or_else(|| {
            "j/k:nav  c:complete  d:delete  n:new  s:sort  f:filter  /:search  ,:settings  q:quit".into()
        }),
    };

    let bar = Paragraph::new(Line::from(Span::styled(
        msg,
        Style::default().fg(Color::White).bg(Color::DarkGray),
    )));

    f.render_widget(bar, area);
}

// ── Create-task screen ──────────────────────────────────────────────────

fn draw_create_screen(f: &mut Frame, app: &App) {
    let chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Length(3), // Title
            Constraint::Length(3), // Priority
            Constraint::Length(3), // Tags
            Constraint::Length(3), // Stack
            Constraint::Min(5),   // Body
            Constraint::Length(1), // Status bar
        ])
        .split(f.size());

    let active_field = match &app.mode {
        InputMode::Creating(f) => f.clone(),
        _ => CreateField::Title,
    };

    let active_style = Style::default().fg(Color::Yellow);
    let normal_style = Style::default();

    // Title
    let title_style = if active_field == CreateField::Title { active_style } else { normal_style };
    let title_block = Block::default()
        .title(" Title ")
        .borders(Borders::ALL)
        .border_style(title_style);
    let cursor = if active_field == CreateField::Title { "_" } else { "" };
    let title_p = Paragraph::new(format!("{}{cursor}", app.create_state.title))
        .block(title_block);
    f.render_widget(title_p, chunks[0]);

    // Priority
    let pri_style = if active_field == CreateField::Priority { active_style } else { normal_style };
    let pri_block = Block::default()
        .title(" Priority (type any key to cycle) ")
        .borders(Borders::ALL)
        .border_style(pri_style);
    let pri_text = match &app.create_state.priority {
        Some(p) => p.to_string(),
        None => "(none)".into(),
    };
    let pri_p = Paragraph::new(pri_text).block(pri_block);
    f.render_widget(pri_p, chunks[1]);

    // Tags
    let tags_style = if active_field == CreateField::Tags { active_style } else { normal_style };
    let tags_block = Block::default()
        .title(" Tags (comma-separated) ")
        .borders(Borders::ALL)
        .border_style(tags_style);
    let cursor = if active_field == CreateField::Tags { "_" } else { "" };
    let tags_p = Paragraph::new(format!("{}{cursor}", app.create_state.tags))
        .block(tags_block);
    f.render_widget(tags_p, chunks[2]);

    // Stack
    let stack_style = if active_field == CreateField::Stack { active_style } else { normal_style };
    let stack_block = Block::default()
        .title(" Stack ")
        .borders(Borders::ALL)
        .border_style(stack_style);
    let cursor = if active_field == CreateField::Stack { "_" } else { "" };
    let stack_p = Paragraph::new(format!("{}{cursor}", app.create_state.stack))
        .block(stack_block);
    f.render_widget(stack_p, chunks[3]);

    // Body
    let body_style = if active_field == CreateField::Body { active_style } else { normal_style };
    let body_block = Block::default()
        .title(" Body ")
        .borders(Borders::ALL)
        .border_style(body_style);
    let cursor = if active_field == CreateField::Body { "_" } else { "" };
    let body_p = Paragraph::new(format!("{}{cursor}", app.create_state.body))
        .block(body_block)
        .wrap(Wrap { trim: false });
    f.render_widget(body_p, chunks[4]);

    // Status bar
    let bar = Paragraph::new(Line::from(Span::styled(
        " Tab=next field  Shift+Tab=prev  Enter=create  Esc=cancel",
        Style::default().fg(Color::White).bg(Color::DarkGray),
    )));
    f.render_widget(bar, chunks[5]);
}

// ── Settings screen ─────────────────────────────────────────────────────

fn draw_settings_screen(f: &mut Frame, app: &App) {
    let chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Min(3),    // Settings list
            Constraint::Length(1), // Status bar
        ])
        .split(f.size());

    let sel = &app.settings_field;
    let s = &app.settings;

    let fields = [
        (SettingsField::DefaultSort, "Default sort", s.default_sort.clone()),
        (
            SettingsField::DefaultFilter,
            "Default filter",
            s.default_filter_status.clone().unwrap_or_else(|| "all".into()),
        ),
        (
            SettingsField::AutoCaptureGit,
            "Auto-capture git",
            if s.auto_capture_git { "on".into() } else { "off".into() },
        ),
        (
            SettingsField::QuickListLimit,
            "Quick list limit",
            s.quick_list_limit.to_string(),
        ),
    ];

    let items: Vec<Line> = fields
        .iter()
        .map(|(field, label, value)| {
            let marker = if field == sel { "> " } else { "  " };
            let style = if field == sel {
                Style::default().fg(Color::Yellow).add_modifier(Modifier::BOLD)
            } else {
                Style::default()
            };
            Line::from(vec![
                Span::styled(format!("{marker}{label}: "), style),
                Span::styled(value.clone(), Style::default().fg(Color::Cyan)),
            ])
        })
        .collect();

    let settings = Paragraph::new(items)
        .block(Block::default().title(" Settings ").borders(Borders::ALL))
        .wrap(Wrap { trim: false });
    f.render_widget(settings, chunks[0]);

    let bar = Paragraph::new(Line::from(Span::styled(
        " j/k=navigate  Enter/Space=toggle  s=save  Esc=back",
        Style::default().fg(Color::White).bg(Color::DarkGray),
    )));
    f.render_widget(bar, chunks[1]);
}
