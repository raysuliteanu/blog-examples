mod tui;

use std::env::args;
use std::error::Error;
use std::fs;
use std::fs::Metadata;
use std::path::Path;

use chrono::{DateTime, Local};
use crossterm::event::KeyCode;
use ratatui::{Frame, text};
use ratatui::layout::{Constraint, Direction, Flex, Layout, Margin};
use ratatui::prelude::{Color, Line, Modifier, Style, Text};
use ratatui::widgets::{Block, Borders, Padding, Paragraph, Scrollbar, ScrollbarOrientation, ScrollbarState, Wrap};

enum Action {
    ScrollUp,
    ScrollDown,
    Home,
    End,
    Quit,
}

struct FileData<'a> {
    path: String,
    data: Vec<Line<'a>>,
    metadata: Metadata,
    vertical_scroll: usize,
    vertical_scroll_state: ScrollbarState,
    quit: bool,
}

#[tokio::main(flavor = "current_thread")]
async fn main() -> Result<(), Box<dyn Error>> {
    let args: Vec<String> = args().collect();

    let mut file_data = get_file_data(args)?;

    let mut tui = tui::Tui::new()?
        .tick_rate(4.0) // 4 ticks per second
        .frame_rate(30.0); // 30 frames per second

    tui.enter()?; // Starts event handler, enters raw mode, enters alternate screen

    loop {

        tui.draw(|f| { // Deref allows calling `tui.terminal.draw`
            ui(f, &mut file_data);
        })?;

        if let Some(evt) = tui.next().await { // `tui.next().await` blocks till next event
            let mut maybe_action = handle_event(evt);
            while let Some(action) = maybe_action {
                maybe_action = update(action, &mut file_data);
            }
        };

        if file_data.quit {
            break;
        }
    }

    tui.exit()?; // stops event handler, exits raw mode, exits alternate screen

    Ok(())
}

fn get_file_data<'a>(args: Vec<String>) -> Result<FileData<'a>, Box<dyn Error>> {
    if args.len() == 2 && !args[1].is_empty() {
        let path = args[1].clone();
        let file_path = Path::new(path.as_str());

        if file_path.exists() && file_path.is_file() {
            let contents = fs::read_to_string(file_path)?;
            let data = contents.split('\n')
                .map(|line| { text::Line::from(line.to_string()) })
                .collect::<Vec<Line>>();
            let vertical_scroll = 0;
            let vertical_scroll_state = ScrollbarState::new(data.len()).position(vertical_scroll);

            let metadata = file_path.metadata().unwrap();
            Ok(FileData {
                path: file_path.display().to_string(),
                data,
                metadata,
                vertical_scroll,
                vertical_scroll_state,
                quit: false,
            })
        } else {
            // todo: return Error
            panic!("file does not exist or cannot be read")
        }
    } else {
        // todo: return Error
        panic!("missing file name argument")
    }
}

fn update(action: Action, file_data: &mut FileData) -> Option<Action> {
    match action {
        Action::ScrollUp => {
            file_data.vertical_scroll = file_data.vertical_scroll.saturating_sub(1);
            file_data.vertical_scroll_state = file_data.vertical_scroll_state.position(file_data.vertical_scroll);
        }
        Action::ScrollDown => {
            file_data.vertical_scroll = file_data.vertical_scroll.saturating_add(1);
            file_data.vertical_scroll_state = file_data.vertical_scroll_state.position(file_data.vertical_scroll);
        }
        Action::Home => {
            file_data.vertical_scroll = 0;
            file_data.vertical_scroll_state = file_data.vertical_scroll_state.position(file_data.vertical_scroll);
        }
        Action::End => {
            file_data.vertical_scroll = file_data.data.len();
            file_data.vertical_scroll_state = file_data.vertical_scroll_state.position(file_data.vertical_scroll);
        }
        Action::Quit => {
            file_data.quit = true;
        }
    }
    None
}

fn handle_event(event: tui::Event) -> Option<Action> {
    if let tui::Event::Key(key) = event {
        return match key.code {
            KeyCode::Up => {
                Some(Action::ScrollUp)
            }
            KeyCode::Down => {
                Some(Action::ScrollDown)
            }
            KeyCode::Home => {
                Some(Action::Home)
            }
            KeyCode::End => {
                Some(Action::End)
            }
            KeyCode::Char('q') | KeyCode::Char('Q') | KeyCode::Esc => {
                Some(Action::Quit)
            }
            _ => {
                None
            }
        }
    }
    None
}


fn ui(frame: &mut Frame, file_data: &mut FileData) {
    let area = frame.size();

    let chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Min(1),
            Constraint::Length(3),
        ])
        .split(area);

    let style_blue_bold = Style::default().fg(Color::Blue).add_modifier(Modifier::BOLD);

    let main_content_block = Block::new()
        .borders(Borders::all())
        .padding(Padding::new(1, 1, 1, 1))
        .title(file_data.path.clone())
        .title_style(style_blue_bold);
    let main_content = Paragraph::new(file_data.data.clone()) // todo: this clone() isn't great
        .scroll((file_data.vertical_scroll as u16, 0))
        .block(main_content_block)
        .wrap(Wrap { trim: false }); // 'trim: false' preserves indenting i.e. no strip whitespace
    frame.render_widget(main_content, chunks[0]);

    let scrollbar = Scrollbar::new(ScrollbarOrientation::VerticalRight);
    frame.render_stateful_widget(
        scrollbar,
        chunks[0].inner(
            // using an inner vertical margin of 1 unit makes the scrollbar inside the block
            &Margin { vertical: 1, horizontal: 0 }
        ),
        &mut file_data.vertical_scroll_state,
    );

    let footer_layout = Layout::default()
        .flex(Flex::SpaceBetween)
        .direction(Direction::Horizontal)
        .constraints([Constraint::Ratio(1, 2), Constraint::Ratio(1, 2)])
        .split(chunks[1]);

    let footer_commands = Text::from("↑ ↓ <Home> <End>");
    let footer_commands_paragraph = Paragraph::new(footer_commands)
        .style(style_blue_bold)
        .left_aligned();
    frame.render_widget(footer_commands_paragraph, footer_layout[0]);

    let system_time = file_data.metadata.created().unwrap();
    let local_time: DateTime<Local> = system_time.into();
    let file_details = format!("Created: {} Length: {}", local_time.format("%d-%m-%Y %H:%M"), file_data.metadata.len());
    let footer_metadata = Text::from(file_details);
    let footer_metadata_paragraph = Paragraph::new(footer_metadata)
        .style(style_blue_bold)
        .right_aligned();
    frame.render_widget(footer_metadata_paragraph, footer_layout[1]);
}
