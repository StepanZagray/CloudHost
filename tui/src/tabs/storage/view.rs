use ratatui::{
    buffer::Buffer,
    layout::{Alignment, Constraint, Direction, Layout, Rect},
    style::{Color, Modifier, Style},
    widgets::{
        Block, Borders, Clear, List, ListItem, Paragraph, Scrollbar, ScrollbarOrientation,
        StatefulWidget, Widget, Wrap,
    },
};

use crate::{
    app::{App, SearchTarget},
    components::{modal::centered_rect, password_modal::render_password_modal},
    tabs::storage::state::{CloudEditFocus, FolderInputField, StoragePanel},
};

pub fn render_storage(app: &mut App, area: Rect, buf: &mut Buffer) {
    if area.width < 20 || area.height < 7 {
        Paragraph::new("Make the terminal a little larger to manage storage.")
            .alignment(Alignment::Center)
            .render(area, buf);
        return;
    }

    let (folders_area, clouds_area, info_area) = storage_layout(area);
    render_folders_list(app, folders_area, buf);
    render_clouds_list(app, clouds_area, buf);
    render_info_panel(app, info_area, buf);

    if app.storage.adding_folder {
        render_folder_modal(app, area, buf, false);
    } else if app.storage.password_creation.creating_password {
        let cloud_name = if app.storage.new_cloud_name.is_empty() {
            app.storage
                .clouds
                .get(app.storage.selected_cloud_index)
                .map(|cloud| cloud.name.as_str())
                .unwrap_or("Selected cloud")
        } else {
            app.storage.new_cloud_name.as_str()
        };
        render_password_modal(
            " Set cloud password ",
            cloud_name,
            &app.storage.password_creation,
            area,
            buf,
        );
    } else if app.storage.creating_cloud {
        render_cloud_creation_modal(app, area, buf);
    } else if app.storage.editing_folder {
        render_folder_modal(app, area, buf, true);
    } else if app.storage.editing_cloud {
        render_cloud_edit_modal(app, area, buf);
    }
}

fn storage_layout(area: Rect) -> (Rect, Rect, Rect) {
    if area.width >= 108 {
        let chunks = Layout::default()
            .direction(Direction::Horizontal)
            .constraints([
                Constraint::Percentage(35),
                Constraint::Percentage(32),
                Constraint::Min(30),
            ])
            .split(area);
        (chunks[0], chunks[1], chunks[2])
    } else if area.width >= 68 {
        let columns = Layout::default()
            .direction(Direction::Horizontal)
            .constraints([Constraint::Percentage(48), Constraint::Min(28)])
            .split(area);
        let lists = Layout::default()
            .direction(Direction::Vertical)
            .constraints([Constraint::Percentage(52), Constraint::Min(4)])
            .split(columns[0]);
        (lists[0], lists[1], columns[1])
    } else {
        let chunks = Layout::default()
            .direction(Direction::Vertical)
            .constraints([
                Constraint::Percentage(34),
                Constraint::Percentage(33),
                Constraint::Min(5),
            ])
            .split(area);
        (chunks[0], chunks[1], chunks[2])
    }
}

fn panel_style(focused: bool, accent: Color) -> Style {
    if focused {
        Style::default().fg(accent).add_modifier(Modifier::BOLD)
    } else {
        Style::default().fg(Color::DarkGray)
    }
}

fn render_folders_list(app: &mut App, area: Rect, buf: &mut Buffer) {
    let target = SearchTarget::StorageFolders;
    let focused = app.storage.focused_panel == StoragePanel::Folders;
    let selected_count = app.storage.selected_folder_count();
    let add = app.config.get_keys_for_action("Add Folder").join(" / ");
    let indices = app.filtered_indices(target);
    let search_suffix = app.search_title_suffix(target);
    let items: Vec<ListItem> = if app.storage.folders.is_empty() {
        vec![ListItem::new(format!(
            "No folders yet\n\nPress {add} to add a local folder."
        ))]
    } else if indices.is_empty() {
        vec![ListItem::new("No folders match this search.")]
    } else {
        indices
            .iter()
            .filter_map(|&index| app.storage.folders.get(index).map(|folder| (index, folder)))
            .map(|(index, folder)| {
                let marked = if app.storage.is_folder_selected(index) {
                    "☑"
                } else {
                    "☐"
                };
                let style = if app.storage.is_folder_selected(index) {
                    Style::default().fg(Color::Cyan)
                } else {
                    Style::default()
                };
                ListItem::new(format!("{marked} {}", folder.name)).style(style)
            })
            .collect()
    };

    let list = List::new(items)
        .block(
            Block::default()
                .borders(Borders::ALL)
                .title(format!(
                    " Folders · {selected_count} selected{search_suffix} "
                ))
                .border_style(panel_style(focused, Color::Green)),
        )
        .highlight_symbol("› ")
        .highlight_style(
            Style::default()
                .fg(Color::White)
                .bg(Color::DarkGray)
                .add_modifier(Modifier::BOLD),
        );

    if app.is_searching(target) {
        let search_selection = app.search_selected_position(target);
        app.storage.folders_list_state.select(search_selection);
    }

    StatefulWidget::render(list, area, buf, &mut app.storage.folders_list_state);

    let selected = app.storage.folders_list_state.selected().unwrap_or(0);
    let mut scroll_state = app
        .storage
        .folders_scroll_state
        .content_length(indices.len())
        .position(selected);
    Scrollbar::new(ScrollbarOrientation::VerticalRight)
        .begin_symbol(Some("↑"))
        .end_symbol(Some("↓"))
        .render(area, buf, &mut scroll_state);
    app.storage.folders_scroll_state = scroll_state;
}

fn render_clouds_list(app: &mut App, area: Rect, buf: &mut Buffer) {
    let target = SearchTarget::StorageClouds;
    let focused = app.storage.focused_panel == StoragePanel::Clouds;
    let create = app.config.get_keys_for_action("Create Cloud").join(" / ");
    let indices = app.filtered_indices(target);
    let search_suffix = app.search_title_suffix(target);
    let items: Vec<ListItem> = if app.storage.clouds.is_empty() {
        vec![ListItem::new(format!(
            "No clouds yet\n\nSelect folders, then press {create} to create one."
        ))]
    } else if indices.is_empty() {
        vec![ListItem::new("No clouds match this search.")]
    } else {
        indices
            .iter()
            .filter_map(|&index| app.storage.clouds.get(index))
            .map(|cloud| {
                let password = if cloud.password.is_some() {
                    "🔒"
                } else {
                    "⚠"
                };
                ListItem::new(format!("{password} {}", cloud.name))
            })
            .collect()
    };

    let list = List::new(items)
        .block(
            Block::default()
                .borders(Borders::ALL)
                .title(format!(
                    " Clouds · {}{search_suffix} ",
                    app.storage.clouds.len()
                ))
                .border_style(panel_style(focused, Color::Blue)),
        )
        .highlight_symbol("› ")
        .highlight_style(
            Style::default()
                .fg(Color::White)
                .bg(Color::DarkGray)
                .add_modifier(Modifier::BOLD),
        );

    if app.is_searching(target) {
        let search_selection = app.search_selected_position(target);
        app.storage.clouds_list_state.select(search_selection);
    }

    StatefulWidget::render(list, area, buf, &mut app.storage.clouds_list_state);

    let selected = app.storage.clouds_list_state.selected().unwrap_or(0);
    let mut scroll_state = app
        .storage
        .clouds_scroll_state
        .content_length(indices.len())
        .position(selected);
    Scrollbar::new(ScrollbarOrientation::VerticalRight)
        .begin_symbol(Some("↑"))
        .end_symbol(Some("↓"))
        .render(area, buf, &mut scroll_state);
    app.storage.clouds_scroll_state = scroll_state;
}

fn render_info_panel(app: &App, area: Rect, buf: &mut Buffer) {
    let (title, accent, text) = match app.storage.focused_panel {
        StoragePanel::Folders => folder_info(app),
        StoragePanel::Clouds => cloud_info(app),
    };

    Paragraph::new(text)
        .block(
            Block::default()
                .borders(Borders::ALL)
                .title(title)
                .border_style(panel_style(true, accent)),
        )
        .wrap(Wrap { trim: true })
        .render(area, buf);
}

fn folder_info(app: &App) -> (String, Color, String) {
    let create = app.config.get_keys_for_action("Add Folder").join(" / ");
    let edit = app.config.get_keys_for_action("Edit").join(" / ");
    let delete = app
        .config
        .get_keys_for_action("Remove Folder / Cloud")
        .join(" / ");
    let toggle = app
        .config
        .get_keys_for_action("Toggle Selection")
        .join(" / ");

    let text = match app.storage.folders.get(app.storage.selected_folder_index) {
        Some(folder) => {
            let selected = if app
                .storage
                .is_folder_selected(app.storage.selected_folder_index)
            {
                "Included in the next cloud"
            } else {
                "Not selected"
            };
            format!(
                "{}\n\n{}\n\n{selected}\n\n{create}  add folder\n{edit}  edit\n{delete}  remove\n{toggle}  include in cloud",
                folder.name,
                folder.folder_path.display(),
            )
        }
        None => format!(
            "Add the first local folder with {create}.\n\nA cloud contains one or more folders."
        ),
    };
    (" Folder details ".to_string(), Color::Green, text)
}

fn cloud_info(app: &App) -> (String, Color, String) {
    let create = app.config.get_keys_for_action("Create Cloud").join(" / ");
    let edit = app.config.get_keys_for_action("Edit").join(" / ");
    let delete = app
        .config
        .get_keys_for_action("Remove Folder / Cloud")
        .join(" / ");
    let password = app
        .config
        .get_keys_for_action("Set Cloud Password")
        .join(" / ");

    let text = match app.storage.clouds.get(app.storage.selected_cloud_index) {
        Some(cloud) => {
            let password_status = cloud
                .password
                .as_ref()
                .map(|value| format!("Set ({})", app.storage.get_password_display(value)))
                .unwrap_or_else(|| "Not set — required before starting".to_string());
            let folders = cloud
                .cloud_folders
                .iter()
                .map(|folder| format!("  • {}", folder.name))
                .collect::<Vec<_>>()
                .join("\n");
            format!(
                "{}\n\nPassword: {password_status}\nFolders ({})\n{}\n\n{create}  create cloud\n{edit}  edit\n{password}  set password\n{delete}  remove",
                cloud.name,
                cloud.cloud_folders.len(),
                folders,
            )
        }
        None if app.storage.selected_folder_count() > 0 => format!(
            "{} folders selected.\n\nPress {create} to make a cloud from them.",
            app.storage.selected_folder_count()
        ),
        None => {
            format!("Select one or more folders first, then press {create} to make a cloud.")
        }
    };
    (" Cloud details ".to_string(), Color::Blue, text)
}

fn render_folder_modal(app: &App, area: Rect, buf: &mut Buffer, editing: bool) {
    let modal_area = centered_rect(area, 64, 15);
    if modal_area.width == 0 || modal_area.height == 0 {
        return;
    }

    let (name, path, field, error, title) = if editing {
        (
            &app.storage.edit_folder_name,
            &app.storage.edit_folder_path,
            app.storage.edit_folder_input_field,
            app.storage.folder_edit_error.as_deref(),
            " Edit folder ",
        )
    } else {
        (
            &app.storage.new_folder_name,
            &app.storage.new_folder_path,
            app.storage.folder_input_field,
            app.storage.folder_creation_error.as_deref(),
            " Add folder ",
        )
    };

    Clear.render(modal_area, buf);
    let chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Length(3),
            Constraint::Length(2),
            Constraint::Length(3),
            Constraint::Length(3),
            Constraint::Min(2),
        ])
        .split(modal_area);

    Paragraph::new("Tab switches fields · Enter continues/saves · Esc cancels")
        .alignment(Alignment::Center)
        .style(Style::default().fg(Color::Cyan))
        .block(
            Block::default()
                .borders(Borders::ALL)
                .title(title)
                .title_alignment(Alignment::Center)
                .border_style(Style::default().fg(Color::Cyan)),
        )
        .render(modal_area, buf);

    render_input_field(
        "Name",
        name,
        field == FolderInputField::Name,
        chunks[2],
        buf,
    );
    render_input_field(
        "Path",
        path,
        field == FolderInputField::Path,
        chunks[3],
        buf,
    );
    render_modal_message(
        error.unwrap_or("Use an existing local directory. Nothing is copied or moved."),
        error.is_some(),
        chunks[4],
        buf,
    );
}

fn render_input_field(label: &str, value: &str, active: bool, area: Rect, buf: &mut Buffer) {
    let style = if active {
        Style::default()
            .fg(Color::Yellow)
            .add_modifier(Modifier::BOLD | Modifier::UNDERLINED)
    } else {
        Style::default().fg(Color::White)
    };
    Paragraph::new(format!("{label}: {value}"))
        .style(style)
        .render(area, buf);
}

fn render_modal_message(message: &str, error: bool, area: Rect, buf: &mut Buffer) {
    Paragraph::new(message)
        .alignment(Alignment::Center)
        .wrap(Wrap { trim: true })
        .style(
            Style::default()
                .fg(if error { Color::Red } else { Color::DarkGray })
                .add_modifier(if error {
                    Modifier::BOLD
                } else {
                    Modifier::empty()
                }),
        )
        .render(area, buf);
}

fn render_cloud_creation_modal(app: &App, area: Rect, buf: &mut Buffer) {
    let modal_area = centered_rect(area, 70, 18);
    if modal_area.width == 0 || modal_area.height == 0 {
        return;
    }
    Clear.render(modal_area, buf);

    let chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Length(3),
            Constraint::Length(2),
            Constraint::Length(3),
            Constraint::Min(4),
            Constraint::Length(2),
        ])
        .split(modal_area);
    Paragraph::new("Name the cloud, then press Enter · Esc cancels")
        .alignment(Alignment::Center)
        .style(Style::default().fg(Color::Cyan))
        .block(
            Block::default()
                .borders(Borders::ALL)
                .title(" Create cloud ")
                .title_alignment(Alignment::Center)
                .border_style(Style::default().fg(Color::Blue)),
        )
        .render(modal_area, buf);
    render_input_field("Name", &app.storage.new_cloud_name, true, chunks[2], buf);

    let names = app.storage.selected_folder_names();
    let selected = if names.is_empty() {
        "No folders selected".to_string()
    } else {
        names
            .iter()
            .map(|name| format!("  • {name}"))
            .collect::<Vec<_>>()
            .join("\n")
    };
    Paragraph::new(selected)
        .block(
            Block::default()
                .borders(Borders::ALL)
                .title(format!(" Included folders · {} ", names.len()))
                .border_style(Style::default().fg(Color::Cyan)),
        )
        .style(Style::default().fg(if names.is_empty() {
            Color::Red
        } else {
            Color::White
        }))
        .render(chunks[3], buf);
    render_modal_message(
        app.storage
            .cloud_creation_error
            .as_deref()
            .unwrap_or("A password will be requested next."),
        app.storage.cloud_creation_error.is_some(),
        chunks[4],
        buf,
    );
}

fn render_cloud_edit_modal(app: &App, area: Rect, buf: &mut Buffer) {
    let target = SearchTarget::CloudEditFolders;
    let modal_area = centered_rect(area, 70, 19);
    if modal_area.width == 0 || modal_area.height == 0 {
        return;
    }
    Clear.render(modal_area, buf);

    let chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Length(3),
            Constraint::Length(2),
            Constraint::Length(3),
            Constraint::Min(5),
            Constraint::Length(2),
        ])
        .split(modal_area);
    let editing_folders = app.storage.cloud_edit_focus == CloudEditFocus::Folders;
    let folder_indices = app.filtered_indices(target);
    let search_suffix = app.search_title_suffix(target);
    let current_folder_index = if app.is_searching(target) {
        app.search_selected_index(target)
    } else {
        Some(app.storage.selected_folder_index)
    };
    let toggle = app
        .config
        .get_keys_for_action("Toggle Selection")
        .join(" / ");
    Paragraph::new(if editing_folders {
        format!(
            "j/k move · / search · {toggle} selects · Tab edits name · Enter saves · Esc cancels"
        )
    } else {
        "Type a name · Tab selects folders · Enter saves · Esc cancels".to_string()
    })
    .alignment(Alignment::Center)
    .style(Style::default().fg(Color::Cyan))
    .block(
        Block::default()
            .borders(Borders::ALL)
            .title(" Edit cloud ")
            .title_alignment(Alignment::Center)
            .border_style(Style::default().fg(Color::Blue)),
    )
    .render(modal_area, buf);
    render_input_field(
        "Name",
        &app.storage.edit_cloud_name,
        !editing_folders,
        chunks[2],
        buf,
    );

    let items: Vec<ListItem> = if folder_indices.is_empty() {
        vec![ListItem::new("No folders match this search.")]
    } else {
        folder_indices
            .iter()
            .filter_map(|&index| app.storage.folders.get(index).map(|folder| (index, folder)))
            .map(|(index, folder)| {
                let selected = app.storage.is_cloud_folder_selected(index);
                let current = current_folder_index == Some(index);
                let mark = if selected { "☑" } else { "☐" };
                let style = if current {
                    Style::default()
                        .fg(Color::Yellow)
                        .add_modifier(Modifier::BOLD)
                } else if selected {
                    Style::default().fg(Color::Cyan)
                } else {
                    Style::default()
                };
                ListItem::new(format!("{mark} {}", folder.name)).style(style)
            })
            .collect()
    };
    Widget::render(
        List::new(items).block(
            Block::default()
                .borders(Borders::ALL)
                .title(format!(
                    " Folders · {} selected{search_suffix} ",
                    app.storage.edit_cloud_selected_folders.len(),
                ))
                .border_style(panel_style(editing_folders, Color::Yellow)),
        ),
        chunks[3],
        buf,
    );
    render_modal_message(
        app.storage
            .cloud_edit_error
            .as_deref()
            .unwrap_or("Choose at least one folder."),
        app.storage.cloud_edit_error.is_some(),
        chunks[4],
        buf,
    );
}
