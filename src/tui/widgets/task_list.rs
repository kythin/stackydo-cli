use crate::model::task::{Priority, Task, TaskStatus};
use ratatui::{
    style::{Color, Modifier, Style},
    text::{Line, Span},
    widgets::ListItem,
};

/// Format a task as a list item for the left pane.
pub fn format_task_line(task: &Task, selected: bool) -> ListItem<'static> {
    let fm = &task.frontmatter;

    let status_icon = match fm.status {
        TaskStatus::Todo => "○",
        TaskStatus::InProgress => "◐",
        TaskStatus::Done => "●",
        TaskStatus::Blocked => "✕",
        TaskStatus::Cancelled => "⊘",
    };

    let status_color = match fm.status {
        TaskStatus::Todo => Color::White,
        TaskStatus::InProgress => Color::Yellow,
        TaskStatus::Done => Color::Green,
        TaskStatus::Blocked => Color::Red,
        TaskStatus::Cancelled => Color::DarkGray,
    };

    let pri_str = match &fm.priority {
        Some(Priority::Critical) => " !!!",
        Some(Priority::High) => " !! ",
        Some(Priority::Medium) => " !  ",
        Some(Priority::Low) => "    ",
        None => "    ",
    };

    let pri_color = match &fm.priority {
        Some(Priority::Critical) => Color::Red,
        Some(Priority::High) => Color::Yellow,
        Some(Priority::Medium) => Color::Blue,
        _ => Color::DarkGray,
    };

    let title = if fm.title.chars().count() > 30 {
        format!("{}...", fm.title.chars().take(27).collect::<String>())
    } else {
        fm.title.clone()
    };

    let due = fm
        .due
        .map(|d| format!(" {}", d.format("%m/%d")))
        .unwrap_or_default();

    let base_style = if selected {
        Style::default()
            .fg(Color::Yellow)
            .add_modifier(Modifier::BOLD)
    } else {
        Style::default()
    };

    ListItem::new(Line::from(vec![
        Span::styled(format!(" {status_icon} "), Style::default().fg(status_color)),
        Span::styled(pri_str.to_string(), Style::default().fg(pri_color)),
        Span::styled(title, base_style),
        Span::styled(due, Style::default().fg(Color::DarkGray)),
    ]))
}
