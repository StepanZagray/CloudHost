use ratatui::{
    buffer::Buffer,
    layout::{Alignment, Constraint, Direction, Layout, Rect},
    style::{Color, Modifier, Style},
    widgets::{
        Block, Borders, Clear, List, ListItem, Paragraph, Scrollbar, ScrollbarOrientation,
        StatefulWidget, Widget,
    },
};

use crate::components::password_modal::render_password_modal;
use crate::models::App;
use crate::tabs::folders::models::FocusedPanel;

pub fn render_folders_tab(app: &App, area: Rect, buf: &mut Buffer) {
    // Create three equal columns: folders, clouds, info
    let chunks = Layout::default()
        .direction(Direction::Horizontal)
        .constraints([
            Constraint::Percentage(33), // Folders
            Constraint::Percentage(33), // Clouds
            Constraint::Percentage(34), // Info
        ])
        .split(area);

    // Render folders list
    render_folders_list(app, chunks[0], buf);

    // Render clouds list
    render_clouds_list(app, chunks[1], buf);

    // Render info panel
    render_info_panel(app, chunks[2], buf);

    // Render creation modals
    if app.folders_state.creating_folder {
        render_folder_creation_modal(app, area, buf);
    } else if app.folders_state.password_creation.creating_password {
        // Determine title and cloud name based on context
        let (title, cloud_name) = if !app.folders_state.new_cloud_name.is_empty() {
            (
                "🔐 Set Cloud Password",
                app.folders_state.new_cloud_name.as_str(),
            )
        } else if app.folders_state.selected_cloud_index < app.folders_state.clouds.len() {
            (
                "🔐 Set Cloud Password",
                app.folders_state.clouds[app.folders_state.selected_cloud_index]
                    .name
                    .as_str(),
            )
        } else {
            ("🔐 Set Cloud Password", "Unknown")
        };

        render_password_modal(
            title,
            cloud_name,
            &app.folders_state.password_creation,
            area,
            buf,
        );
    } else if app.folders_state.creating_cloud {
        render_cloud_creation_modal(app, area, buf);
    }

    // Render edit modals
    if app.folders_state.editing_folder {
        render_folder_edit_modal(app, area, buf);
    } else if app.folders_state.editing_cloud {
        render_cloud_edit_modal(app, area, buf);
    }
}

fn render_folders_list(app: &App, area: Rect, buf: &mut Buffer) {
    let selected_count = app.folders_state.get_selected_folders_count();
    let title = if app.folders_state.focused_panel == FocusedPanel::Folders {
        format!("Cloud Folders (FOCUSED) - Selected: {}", selected_count)
    } else {
        format!("Cloud Folders - Selected: {}", selected_count)
    };

    let block = Block::default()
        .borders(Borders::ALL)
        .title(title)
        .border_style(
            Style::default()
                .fg(Color::Green)
                .add_modifier(ratatui::style::Modifier::BOLD),
        );

    let folders_items: Vec<ListItem> = app
        .folders_state
        .cloud_folders
        .iter()
        .enumerate()
        .map(|(i, folder)| {
            let is_selected = app.folders_state.is_folder_selected(i);
            let radio_indicator = if is_selected { "●" } else { "○" };

            let style = if i == app.folders_state.selected_folder_index {
                Style::default().fg(Color::Yellow)
            } else if is_selected {
                Style::default().fg(Color::Cyan)
            } else {
                Style::default()
            };

            let content = format!("{} {}", radio_indicator, folder.name);
            ListItem::new(content).style(style)
        })
        .collect();

    let folders_list = List::new(folders_items).block(block);

    // Use StatefulWidget for proper scrolling
    StatefulWidget::render(
        folders_list,
        area,
        buf,
        &mut app.folders_state.folders_list_state.clone(),
    );

    // Render scrollbar
    let mut scroll_state = app.folders_state.folders_scroll_state;
    scroll_state = scroll_state.content_length(app.folders_state.cloud_folders.len());
    if let Some(selected) = app.folders_state.folders_list_state.selected() {
        scroll_state = scroll_state.position(selected);
    }

    let scrollbar = Scrollbar::new(ScrollbarOrientation::VerticalRight)
        .begin_symbol(Some("↑"))
        .end_symbol(Some("↓"));

    scrollbar.render(area, buf, &mut scroll_state);
}

fn render_clouds_list(app: &App, area: Rect, buf: &mut Buffer) {
    let title = if app.folders_state.focused_panel == FocusedPanel::Clouds {
        "Clouds (FOCUSED)"
    } else {
        "Clouds"
    };

    let block = Block::default()
        .borders(Borders::ALL)
        .title(title)
        .border_style(
            Style::default()
                .fg(Color::Blue)
                .add_modifier(ratatui::style::Modifier::BOLD),
        );

    let clouds_items: Vec<ListItem> = app
        .folders_state
        .clouds
        .iter()
        .enumerate()
        .map(|(i, cloud)| {
            let style = if i == app.folders_state.selected_cloud_index {
                Style::default().fg(Color::Yellow)
            } else {
                Style::default()
            };
            ListItem::new(cloud.name.as_str()).style(style)
        })
        .collect();

    let clouds_list = List::new(clouds_items).block(block);

    // Use StatefulWidget for proper scrolling
    StatefulWidget::render(
        clouds_list,
        area,
        buf,
        &mut app.folders_state.clouds_list_state.clone(),
    );

    // Render scrollbar
    let mut scroll_state = app.folders_state.clouds_scroll_state;
    scroll_state = scroll_state.content_length(app.folders_state.clouds.len());
    if let Some(selected) = app.folders_state.clouds_list_state.selected() {
        scroll_state = scroll_state.position(selected);
    }

    let scrollbar = Scrollbar::new(ScrollbarOrientation::VerticalRight)
        .begin_symbol(Some("↑"))
        .end_symbol(Some("↓"));

    scrollbar.render(area, buf, &mut scroll_state);
}

fn render_info_panel(app: &App, area: Rect, buf: &mut Buffer) {
    let title = if app.folders_state.focused_panel == FocusedPanel::Info {
        "Info (FOCUSED)"
    } else {
        "Info"
    };

    let block = Block::default()
        .borders(Borders::ALL)
        .title(title)
        .border_style(
            Style::default()
                .fg(Color::Magenta)
                .add_modifier(ratatui::style::Modifier::BOLD),
        );

    let info_text = match app.folders_state.focused_panel {
        FocusedPanel::Folders => {
            if app.folders_state.cloud_folders.is_empty() {
                "No cloud folders available.\n\nPress 'n' to create a new cloud folder.".to_string()
            } else if app.folders_state.selected_folder_index
                < app.folders_state.cloud_folders.len()
            {
                let folder =
                    &app.folders_state.cloud_folders[app.folders_state.selected_folder_index];
                let is_selected = app
                    .folders_state
                    .is_folder_selected(app.folders_state.selected_folder_index);
                let selection_status = if is_selected {
                    "SELECTED"
                } else {
                    "Not selected"
                };
                // Get keybinds dynamically from config
                let create_keys = app.config.get_keys_for_action("Create New").join(", ");
                let delete_keys = app.config.get_keys_for_action("Delete Folder").join(", ");
                let toggle_keys = app
                    .config
                    .get_keys_for_action("Toggle Selection")
                    .join(", ");
                let select_all_keys = app
                    .config
                    .get_keys_for_action("Select All Folders")
                    .join(", ");

                format!(
                    "Cloud Folder: {}\nPath: {}\nStatus: {}\n\nPress {} to create cloud folder.\nPress {} to delete this cloud folder.\nPress {} to toggle selection.\nPress {} to select all cloud folders.",
                    folder.name,
                    folder.folder_path.display(),
                    selection_status,
                    create_keys,
                    delete_keys,
                    toggle_keys,
                    select_all_keys,
                )
            } else {
                "No cloud folder selected.".to_string()
            }
        }
        FocusedPanel::Clouds => {
            let selected_count = app.folders_state.get_selected_folders_count();
            if app.folders_state.clouds.is_empty() {
                // Get keybinds dynamically from config
                let toggle_keys = app.config.get_keys_for_action("Toggle Selection");
                let create_keys = app.config.get_keys_for_action("Create New");

                if selected_count == 0 {
                    format!("⚠️  No cloud folders selected!\n\nTo create a cloud:\n1. Select cloud folders using {} key\n2. Press {} to create cloud\n\nCurrently selected: 0 folders", 
                        toggle_keys.join(", "), create_keys.join(", "))
                } else {
                    format!("✅ {} cloud folder(s) selected.\n\nPress {} to create a cloud with selected folders.\nAfter creation, you'll set a password for security.\n\nSelected folders: {}", 
                        selected_count,
                        create_keys.join(", "),
                        app.folders_state.get_selected_folder_names().join(", ")
                    )
                }
            } else if app.folders_state.selected_cloud_index < app.folders_state.clouds.len() {
                let cloud = &app.folders_state.clouds[app.folders_state.selected_cloud_index];
                let password_display = if let Some(ref password) = cloud.password {
                    app.folders_state.get_password_display(password)
                } else {
                    "No password set".to_string()
                };
                let password_status = if cloud.password.is_some() {
                    ""
                } else {
                    "❌ Not set"
                };
                // Get keybinds dynamically from config
                let create_keys = app.config.get_keys_for_action("Create New").join(", ");
                let edit_keys = app.config.get_keys_for_action("Edit").join(", ");
                let delete_keys = app.config.get_keys_for_action("Delete Cloud").join(", ");
                let password_keys = app.config.get_keys_for_action("Set Password").join(", ");
                let toggle_visibility_keys = app
                    .config
                    .get_keys_for_action("Toggle Password Visibility")
                    .join(", ");

                format!(
                    "Cloud: {}\nPassword: {}{}\n\nPress {} to create new cloud.\nPress {} to edit this cloud.\nPress {} to delete this cloud.\nPress {} to set password for this cloud.\nPress {} to toggle password visibility.\n\nCloud Folders ({}):\n{}",
                    cloud.name,
                    password_display,
                    password_status,
                    create_keys,
                    edit_keys,
                    delete_keys,
                    password_keys,
                    toggle_visibility_keys,
                    cloud.cloud_folders.len(),
                    cloud.cloud_folders.iter().map(|folder| format!("• {}", folder.name)).collect::<Vec<String>>().join("\n"),
                )
            } else {
                "No cloud selected.".to_string()
            }
        }
        FocusedPanel::Info => {
            "ℹ️  Info Panel\n\nThis panel shows information about the selected item.".to_string()
        }
    };

    let info_paragraph = Paragraph::new(info_text)
        .block(block)
        .wrap(ratatui::widgets::Wrap { trim: true });

    info_paragraph.render(area, buf);
}

fn render_folder_creation_modal(app: &App, area: Rect, buf: &mut Buffer) {
    // Create a centered modal
    let modal_width = 60;
    let modal_height = 16;
    let x = (area.width.saturating_sub(modal_width)) / 2;
    let y = (area.height.saturating_sub(modal_height)) / 2;

    let modal_area = Rect::new(area.x + x, area.y + y, modal_width, modal_height);

    // Clear the modal area
    Clear.render(modal_area, buf);

    // Create modal layout
    let modal_chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Length(3), // Title
            Constraint::Length(2), // Instructions
            Constraint::Length(3), // Name field
            Constraint::Length(3), // Path field
            Constraint::Length(2), // Error/Help
            Constraint::Min(0),    // Spacer
        ])
        .split(modal_area);

    // Modal title
    let title_block = Block::default()
        .borders(Borders::ALL)
        .title("📁 Create New Cloud Folder")
        .title_alignment(Alignment::Center)
        .border_style(
            Style::default()
                .fg(Color::Yellow)
                .add_modifier(Modifier::BOLD),
        );

    Paragraph::new("")
        .block(title_block)
        .render(modal_area, buf);

    // Instructions
    Paragraph::new("Tab to switch fields, Enter to submit, Esc to cancel")
        .style(Style::default().fg(Color::Cyan))
        .alignment(Alignment::Center)
        .render(modal_chunks[1], buf);

    // Name field
    let name_style = if matches!(
        app.folders_state.folder_input_field,
        crate::tabs::folders::models::FolderInputField::Name
    ) {
        Style::default()
            .fg(Color::Green)
            .add_modifier(Modifier::BOLD)
    } else {
        Style::default().fg(Color::White)
    };

    let name_text = format!("Name: {}", app.folders_state.new_folder_name);
    Paragraph::new(name_text)
        .style(name_style)
        .alignment(Alignment::Left)
        .render(modal_chunks[2], buf);

    // Path field
    let path_style = if matches!(
        app.folders_state.folder_input_field,
        crate::tabs::folders::models::FolderInputField::Path
    ) {
        Style::default()
            .fg(Color::Green)
            .add_modifier(Modifier::BOLD)
    } else {
        Style::default().fg(Color::White)
    };

    let path_text = format!("Path: {}", app.folders_state.new_folder_path);
    Paragraph::new(path_text)
        .style(path_style)
        .alignment(Alignment::Left)
        .render(modal_chunks[3], buf);

    // Error message or help text
    if let Some(ref error) = app.folders_state.folder_creation_error {
        Paragraph::new(format!("❌ {}", error))
            .style(Style::default().fg(Color::Red).add_modifier(Modifier::BOLD))
            .alignment(Alignment::Center)
            .render(modal_chunks[4], buf);
    } else {
        Paragraph::new("Fill in both fields and press Enter")
            .style(Style::default().fg(Color::Gray))
            .alignment(Alignment::Center)
            .render(modal_chunks[4], buf);
    }
}

fn render_cloud_creation_modal(app: &App, area: Rect, buf: &mut Buffer) {
    // Create a centered modal
    let modal_width = 70;
    let modal_height = 20;
    let x = (area.width.saturating_sub(modal_width)) / 2;
    let y = (area.height.saturating_sub(modal_height)) / 2;

    let modal_area = Rect::new(area.x + x, area.y + y, modal_width, modal_height);

    // Clear the modal area
    Clear.render(modal_area, buf);

    // Create modal layout
    let modal_chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Length(3), // Title
            Constraint::Length(2), // Instructions
            Constraint::Length(3), // Name field
            Constraint::Min(5),    // Selected folders list
            Constraint::Length(2), // Error/Help
        ])
        .split(modal_area);

    // Modal title
    let title_block = Block::default()
        .borders(Borders::ALL)
        .title("☁️  Create New Cloud")
        .title_alignment(Alignment::Center)
        .border_style(
            Style::default()
                .fg(Color::Yellow)
                .add_modifier(Modifier::BOLD),
        );

    Paragraph::new("")
        .block(title_block)
        .render(modal_area, buf);

    // Instructions
    Paragraph::new("Enter cloud name, then press Enter. Esc to cancel")
        .style(Style::default().fg(Color::Cyan))
        .alignment(Alignment::Center)
        .render(modal_chunks[1], buf);

    // Name field
    let name_text = format!("Name: {}", app.folders_state.new_cloud_name);
    Paragraph::new(name_text)
        .style(
            Style::default()
                .fg(Color::Green)
                .add_modifier(Modifier::BOLD),
        )
        .alignment(Alignment::Left)
        .render(modal_chunks[2], buf);

    // Selected folders list
    let selected_folder_names = app.folders_state.get_selected_folder_names();
    let selected_count = selected_folder_names.len();

    let folders_block = Block::default()
        .borders(Borders::ALL)
        .title(format!("Selected Cloud Folders ({})", selected_count))
        .border_style(Style::default().fg(Color::Cyan));

    if selected_folder_names.is_empty() {
        Paragraph::new(
            "⚠️  NO CLOUD FOLDERS SELECTED!\n\nTo create a cloud:\n1. Press Esc to close this modal\n2. Select cloud folders using <leader> key\n3. Press 'n' again to create cloud\n\nCurrently selected: 0 folders",
        )
        .block(folders_block)
        .style(Style::default().fg(Color::Red).add_modifier(Modifier::BOLD))
        .alignment(Alignment::Center)
        .render(modal_chunks[3], buf);
    } else {
        let folder_items: Vec<ListItem> = selected_folder_names
            .iter()
            .map(|name| ListItem::new(format!("• {}", name)))
            .collect();

        let folders_list = List::new(folder_items)
            .block(folders_block)
            .style(Style::default().fg(Color::White));

        Widget::render(folders_list, modal_chunks[3], buf);
    }

    // Error message or help text
    if let Some(ref error) = app.folders_state.cloud_creation_error {
        Paragraph::new(format!("❌ {}", error))
            .style(Style::default().fg(Color::Red).add_modifier(Modifier::BOLD))
            .alignment(Alignment::Center)
            .render(modal_chunks[4], buf);
    } else if selected_count > 0 {
        Paragraph::new(format!(
            "Creating cloud with {} cloud folder(s).\nAfter creation, you'll set a password.\nPress Enter to confirm.",
            selected_count
        ))
        .style(Style::default().fg(Color::Gray))
        .alignment(Alignment::Center)
        .render(modal_chunks[4], buf);
    }
}

fn render_folder_edit_modal(app: &App, area: Rect, buf: &mut Buffer) {
    // Create a centered modal
    let modal_width = 60;
    let modal_height = 16;
    let x = (area.width.saturating_sub(modal_width)) / 2;
    let y = (area.height.saturating_sub(modal_height)) / 2;

    let modal_area = Rect::new(area.x + x, area.y + y, modal_width, modal_height);

    // Clear the modal area
    Clear.render(modal_area, buf);

    // Create modal layout
    let modal_chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Length(3), // Title
            Constraint::Length(2), // Instructions
            Constraint::Length(3), // Name field
            Constraint::Length(3), // Path field
            Constraint::Length(2), // Error/Help
            Constraint::Min(0),    // Spacer
        ])
        .split(modal_area);

    // Modal title
    let title_block = Block::default()
        .borders(Borders::ALL)
        .title("✏️  Edit Cloud Folder")
        .title_alignment(Alignment::Center)
        .border_style(
            Style::default()
                .fg(Color::Yellow)
                .add_modifier(Modifier::BOLD),
        );

    Paragraph::new("")
        .block(title_block)
        .render(modal_area, buf);

    // Instructions
    Paragraph::new("Tab to switch fields, Enter to save, Esc to cancel")
        .style(Style::default().fg(Color::Cyan))
        .alignment(Alignment::Center)
        .render(modal_chunks[1], buf);

    // Name field
    let name_style = if matches!(
        app.folders_state.edit_folder_input_field,
        crate::tabs::folders::models::FolderInputField::Name
    ) {
        Style::default()
            .fg(Color::Green)
            .add_modifier(Modifier::BOLD)
    } else {
        Style::default().fg(Color::White)
    };

    let name_text = format!("Name: {}", app.folders_state.edit_folder_name);
    Paragraph::new(name_text)
        .style(name_style)
        .alignment(Alignment::Left)
        .render(modal_chunks[2], buf);

    // Path field
    let path_style = if matches!(
        app.folders_state.edit_folder_input_field,
        crate::tabs::folders::models::FolderInputField::Path
    ) {
        Style::default()
            .fg(Color::Green)
            .add_modifier(Modifier::BOLD)
    } else {
        Style::default().fg(Color::White)
    };

    let path_text = format!("Path: {}", app.folders_state.edit_folder_path);
    Paragraph::new(path_text)
        .style(path_style)
        .alignment(Alignment::Left)
        .render(modal_chunks[3], buf);

    // Error message or help text
    if let Some(ref error) = app.folders_state.folder_edit_error {
        Paragraph::new(format!("❌ {}", error))
            .style(Style::default().fg(Color::Red).add_modifier(Modifier::BOLD))
            .alignment(Alignment::Center)
            .render(modal_chunks[4], buf);
    } else {
        Paragraph::new("Edit both fields and press Enter to save")
            .style(Style::default().fg(Color::Gray))
            .alignment(Alignment::Center)
            .render(modal_chunks[4], buf);
    }
}

fn render_cloud_edit_modal(app: &App, area: Rect, buf: &mut Buffer) {
    // Create a centered modal
    let modal_width = 70;
    let modal_height = 20;
    let x = (area.width.saturating_sub(modal_width)) / 2;
    let y = (area.height.saturating_sub(modal_height)) / 2;

    let modal_area = Rect::new(area.x + x, area.y + y, modal_width, modal_height);

    // Clear the modal area
    Clear.render(modal_area, buf);

    // Create modal layout
    let modal_chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Length(3), // Title
            Constraint::Length(2), // Instructions
            Constraint::Length(3), // Name field
            Constraint::Min(5),    // Folders list
            Constraint::Length(2), // Error/Help
        ])
        .split(modal_area);

    // Modal title
    let title_block = Block::default()
        .borders(Borders::ALL)
        .title("✏️  Edit Cloud")
        .title_alignment(Alignment::Center)
        .border_style(
            Style::default()
                .fg(Color::Yellow)
                .add_modifier(Modifier::BOLD),
        );

    Paragraph::new("")
        .block(title_block)
        .render(modal_area, buf);

    // Instructions
    let instructions = match app.folders_state.cloud_edit_focus {
        crate::tabs::folders::models::CloudEditFocus::Name => {
            "📝 Editing name: Type to edit, Tab to switch to folders, Enter to save, Esc to cancel"
                .to_string()
        }
        crate::tabs::folders::models::CloudEditFocus::Folders => {
            // Get keybinds dynamically from config
            let toggle_keys = app
                .config
                .get_keys_for_action("Toggle Selection")
                .join(", ");
            format!("📁 Navigating folders: j/k to navigate, {} to toggle, Tab to switch to name, Enter to save, Esc to cancel", toggle_keys)
        }
    };
    Paragraph::new(instructions)
        .style(Style::default().fg(Color::Cyan))
        .alignment(Alignment::Center)
        .render(modal_chunks[1], buf);

    // Name field
    let name_text = format!("Name: {}", app.folders_state.edit_cloud_name);
    let name_style = if app.folders_state.cloud_edit_focus
        == crate::tabs::folders::models::CloudEditFocus::Name
    {
        Style::default()
            .fg(Color::Yellow)
            .add_modifier(Modifier::BOLD | Modifier::UNDERLINED)
    } else {
        Style::default()
            .fg(Color::Green)
            .add_modifier(Modifier::BOLD)
    };
    Paragraph::new(name_text)
        .style(name_style)
        .alignment(Alignment::Left)
        .render(modal_chunks[2], buf);

    // Folders list
    let selected_count = app.folders_state.edit_cloud_selected_folders.len();

    let folders_block = Block::default()
        .borders(Borders::ALL)
        .title(format!("Folders ({} selected)", selected_count))
        .border_style(
            if app.folders_state.cloud_edit_focus
                == crate::tabs::folders::models::CloudEditFocus::Folders
            {
                Style::default()
                    .fg(Color::Yellow)
                    .add_modifier(Modifier::BOLD)
            } else {
                Style::default().fg(Color::Cyan)
            },
        );

    let folder_items: Vec<ListItem> = app
        .folders_state
        .cloud_folders
        .iter()
        .enumerate()
        .map(|(index, folder)| {
            let is_selected = app.folders_state.is_cloud_folder_selected(index);
            let radio_indicator = if is_selected { "●" } else { "○" };

            let style = if index == app.folders_state.selected_folder_index {
                Style::default().fg(Color::Yellow)
            } else if is_selected {
                Style::default().fg(Color::Cyan)
            } else {
                Style::default()
            };

            let content = format!("{} {}", radio_indicator, folder.name);
            ListItem::new(content).style(style)
        })
        .collect();

    let folders_list = List::new(folder_items).block(folders_block);

    Widget::render(folders_list, modal_chunks[3], buf);

    // Error message or help text
    if let Some(ref error) = app.folders_state.cloud_edit_error {
        Paragraph::new(format!("❌ {}", error))
            .style(Style::default().fg(Color::Red).add_modifier(Modifier::BOLD))
            .alignment(Alignment::Center)
            .render(modal_chunks[4], buf);
    } else if selected_count > 0 {
        Paragraph::new(format!(
            "Editing cloud with {} folder(s). Press Enter to save.",
            selected_count
        ))
        .style(Style::default().fg(Color::Gray))
        .alignment(Alignment::Center)
        .render(modal_chunks[4], buf);
    }
}
