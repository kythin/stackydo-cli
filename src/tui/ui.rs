use crate::model::task::Priority;
use crate::tui::app::{App, InputMode};
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
    let chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Min(3),    // Main area
            Constraint::Length(1), // Status bar
        ])
        .split(f.size());

    let main = Layout::default()
        .direction(Direction::Horizontal)
        .constraints([Constraint::Percentage(40), Constraint::Percentage(60)])
        .split(chunks[0]);

    draw_task_list(f, app, main[0]);
    draw_detail_pane(f, app, main[1]);
    draw_status_bar(f, app, chunks[1]);
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
            "j/k:nav  c:complete  d:delete  s:sort  f:filter  /:search  q:quit".into()
        }),
    };

    let bar = Paragraph::new(Line::from(Span::styled(
        msg,
        Style::default().fg(Color::White).bg(Color::DarkGray),
    )));

    f.render_widget(bar, area);
}
