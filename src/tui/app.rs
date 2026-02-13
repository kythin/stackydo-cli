use crate::model::task::{Priority, Task, TaskStatus};
use crate::storage::task_store::TaskStore;

/// Which pane/mode the TUI is in
#[derive(Debug, Clone, PartialEq)]
pub enum InputMode {
    /// Normal navigation
    Normal,
    /// Typing a search query
    Searching,
    /// Filter menu open
    FilterMenu,
}

/// Sort field options
#[derive(Debug, Clone, PartialEq)]
pub enum SortField {
    Created,
    Due,
    Priority,
    Modified,
}

impl SortField {
    pub fn next(&self) -> Self {
        match self {
            Self::Created => Self::Due,
            Self::Due => Self::Priority,
            Self::Priority => Self::Modified,
            Self::Modified => Self::Created,
        }
    }

    pub fn label(&self) -> &str {
        match self {
            Self::Created => "Created",
            Self::Due => "Due",
            Self::Priority => "Priority",
            Self::Modified => "Modified",
        }
    }
}

/// Filter state
#[derive(Debug, Clone, Default)]
pub struct FilterState {
    pub status: Option<TaskStatus>,
    pub tag: Option<String>,
    pub priority: Option<Priority>,
}

/// Core TUI application state
pub struct App {
    /// All tasks loaded from disk
    pub all_tasks: Vec<Task>,

    /// Filtered + sorted view
    pub visible_tasks: Vec<usize>, // indices into all_tasks

    /// Selected index in visible_tasks
    pub selected: usize,

    /// Current input mode
    pub mode: InputMode,

    /// Search buffer
    pub search_input: String,

    /// Active search query (applied after Enter)
    pub search_query: Option<String>,

    /// Sort configuration
    pub sort_field: SortField,
    pub sort_reverse: bool,

    /// Filter state
    pub filters: FilterState,

    /// Whether the app should quit
    pub should_quit: bool,

    /// Status message (bottom bar)
    pub status_msg: Option<String>,
}

impl App {
    pub fn new() -> Self {
        Self {
            all_tasks: Vec::new(),
            visible_tasks: Vec::new(),
            selected: 0,
            mode: InputMode::Normal,
            search_input: String::new(),
            search_query: None,
            sort_field: SortField::Created,
            sort_reverse: false,
            filters: FilterState::default(),
            should_quit: false,
            status_msg: None,
        }
    }

    /// Load all tasks from disk
    pub fn load_tasks(&mut self) -> crate::error::Result<()> {
        let store = TaskStore::new();
        self.all_tasks = store.load_all()?;
        self.refresh_view();
        Ok(())
    }

    /// Re-apply filters, search, and sort to rebuild visible_tasks
    pub fn refresh_view(&mut self) {
        let mut indices: Vec<usize> = (0..self.all_tasks.len()).collect();

        // Filter by status
        if let Some(ref status) = self.filters.status {
            indices.retain(|&i| &self.all_tasks[i].frontmatter.status == status);
        }

        // Filter by tag
        if let Some(ref tag) = self.filters.tag {
            let tag_lower = tag.to_lowercase();
            indices.retain(|&i| {
                self.all_tasks[i]
                    .frontmatter
                    .tags
                    .iter()
                    .any(|t| t.to_lowercase() == tag_lower)
            });
        }

        // Filter by priority
        if let Some(ref pri) = self.filters.priority {
            indices.retain(|&i| self.all_tasks[i].frontmatter.priority.as_ref() == Some(pri));
        }

        // Search filter
        if let Some(ref query) = self.search_query {
            let q = query.to_lowercase();
            indices.retain(|&i| {
                let t = &self.all_tasks[*&i];
                t.frontmatter.title.to_lowercase().contains(&q)
                    || t.body.to_lowercase().contains(&q)
            });
        }

        // Sort
        let tasks = &self.all_tasks;
        indices.sort_by(|&a, &b| {
            let ta = &tasks[a].frontmatter;
            let tb = &tasks[b].frontmatter;
            let cmp = match self.sort_field {
                SortField::Created => tb.created.cmp(&ta.created),
                SortField::Modified => tb.modified.cmp(&ta.modified),
                SortField::Due => ta.due.cmp(&tb.due),
                SortField::Priority => ta.priority.cmp(&tb.priority),
            };
            if self.sort_reverse {
                cmp.reverse()
            } else {
                cmp
            }
        });

        self.visible_tasks = indices;

        // Clamp selection
        if self.selected >= self.visible_tasks.len() {
            self.selected = self.visible_tasks.len().saturating_sub(1);
        }
    }

    /// Get the currently selected task (if any)
    pub fn selected_task(&self) -> Option<&Task> {
        self.visible_tasks
            .get(self.selected)
            .map(|&idx| &self.all_tasks[idx])
    }

    /// Move selection up
    pub fn select_prev(&mut self) {
        self.selected = self.selected.saturating_sub(1);
    }

    /// Move selection down
    pub fn select_next(&mut self) {
        if !self.visible_tasks.is_empty() {
            self.selected = (self.selected + 1).min(self.visible_tasks.len() - 1);
        }
    }

    /// Toggle sort field
    pub fn cycle_sort(&mut self) {
        self.sort_field = self.sort_field.next();
        self.refresh_view();
        self.status_msg = Some(format!("Sort: {}", self.sort_field.label()));
    }

    /// Toggle sort direction
    pub fn toggle_sort_reverse(&mut self) {
        self.sort_reverse = !self.sort_reverse;
        self.refresh_view();
    }

    /// Mark the selected task as done and save
    pub fn complete_selected(&mut self) -> crate::error::Result<()> {
        if let Some(&idx) = self.visible_tasks.get(self.selected) {
            self.all_tasks[idx].frontmatter.status = TaskStatus::Done;
            self.all_tasks[idx].frontmatter.modified = chrono::Utc::now();
            let store = TaskStore::new();
            store.save(&self.all_tasks[idx])?;
            self.status_msg = Some("Task completed.".into());
            self.refresh_view();
        }
        Ok(())
    }

    /// Delete the selected task
    pub fn delete_selected(&mut self) -> crate::error::Result<()> {
        if let Some(&idx) = self.visible_tasks.get(self.selected) {
            let id = self.all_tasks[idx].frontmatter.id.clone();
            let store = TaskStore::new();
            store.delete(&id)?;
            self.all_tasks.remove(idx);
            self.status_msg = Some("Task deleted.".into());
            self.refresh_view();
        }
        Ok(())
    }

    /// Apply search
    pub fn apply_search(&mut self) {
        if self.search_input.trim().is_empty() {
            self.search_query = None;
        } else {
            self.search_query = Some(self.search_input.clone());
        }
        self.refresh_view();
    }

    /// Clear search
    pub fn clear_search(&mut self) {
        self.search_input.clear();
        self.search_query = None;
        self.refresh_view();
    }

    /// Cycle status filter: None -> Todo -> InProgress -> Done -> Blocked -> Cancelled -> None
    pub fn cycle_status_filter(&mut self) {
        self.filters.status = match &self.filters.status {
            None => Some(TaskStatus::Todo),
            Some(TaskStatus::Todo) => Some(TaskStatus::InProgress),
            Some(TaskStatus::InProgress) => Some(TaskStatus::Done),
            Some(TaskStatus::Done) => Some(TaskStatus::Blocked),
            Some(TaskStatus::Blocked) => Some(TaskStatus::Cancelled),
            Some(TaskStatus::Cancelled) => None,
        };
        self.refresh_view();
        self.status_msg = Some(match &self.filters.status {
            None => "Filter: all".into(),
            Some(s) => format!("Filter: {s}"),
        });
    }
}
