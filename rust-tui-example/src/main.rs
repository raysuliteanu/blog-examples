use std::cmp::PartialEq;
use std::env::args;
use std::error::Error;
use std::fs::{File, Metadata};
use std::io::{BufRead, BufReader};
use std::path::Path;

use chrono::{DateTime, Local};
use crossterm::event::KeyCode;
use ratatui::Frame;
use ratatui::layout::{Constraint, Direction, Flex, Layout, Margin};
use ratatui::prelude::{Color, Modifier, Style, Text};
use ratatui::text::Line;
use ratatui::widgets::{Block, Borders, Padding, Paragraph, Scrollbar, ScrollbarOrientation, ScrollbarState, Wrap};

mod tui;

#[derive(PartialEq, Clone, Copy)]
enum Action {
    ScrollUp,
    ScrollDown,
    Home,
    End,
    Quit,
}

struct ScrollState {
    state: ScrollbarState,
    position: usize,
}

struct FileData {
    path: String,
    data: Vec<Line<'static>>,
    metadata: Metadata,
    action: Option<Action>,
    scroll_state: ScrollState,
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
            let some_action = map_event(evt);
            file_data.action = some_action;

            if is_quit_action(&mut file_data) {
                break;
            }
        };
    }

    tui.exit()?; // stops event handler, exits raw mode, exits alternate screen

    Ok(())
}

fn is_quit_action(file_data: &mut FileData) -> bool {
    file_data.action.is_some_and(|action| action == Action::Quit)
}

fn get_file_data(args: Vec<String>) -> Result<FileData, Box<dyn Error>> {
    if args.len() == 2 && !args[1].is_empty() {
        let path = args[1].clone();
        let file_path = Path::new(path.as_str());

        if file_path.exists() && file_path.is_file() {
            let file = File::open(file_path).unwrap();
            let reader = BufReader::new(file);
            let data : Vec<Line> = reader.lines()
                .map(|line| { Line::from(line.unwrap()) })
                .collect::<Vec<_>>();

            let scroll_state = ScrollState {
                state: ScrollbarState::new(data.len()),
                position: 0,
            };

            let metadata = file_path.metadata().unwrap();
            Ok(FileData {
                path: file_path.to_str().unwrap().to_string(),
                data,
                metadata,
                action: None,
                scroll_state,
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

fn map_event(event: tui::Event) -> Option<Action> {
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
        };
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

    update_scroll_state(file_data);

    let text = file_data.data.to_vec();
    let main_content = Paragraph::new(text)
        .scroll((file_data.scroll_state.position  as u16, 0))
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
        &mut file_data.scroll_state.state,
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

fn update_scroll_state(file_data: &mut FileData) {
    if let Some(action) = file_data.action {
        match action {
            Action::ScrollUp => {
                file_data.scroll_state.state.prev();
                file_data.scroll_state.position =
                    file_data.scroll_state.position.saturating_sub(1);
            }
            Action::ScrollDown => {
                file_data.scroll_state.state.next();
                file_data.scroll_state.position =
                    file_data.scroll_state.position.saturating_add(1);
            }
            Action::Home => {
                file_data.scroll_state.state.first();
                file_data.scroll_state.position = 0;
            }
            Action::End => {
                file_data.scroll_state.position = file_data.data.len();
                let _ = file_data.scroll_state.state.position(file_data.scroll_state.position);
            }
            _ => {}
        }

        // reset otherwise keep doing same action till some other action from the user!
        file_data.action = None;
    }
}
