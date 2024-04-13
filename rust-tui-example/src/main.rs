use std::{fs, io};
use std::default::Default;
use std::env::args;
use std::error::Error;
use std::fs::Metadata;
use std::io::{stdout, Stdout};
use std::panic::{set_hook, take_hook};
use std::path::Path;
use std::time::Duration;

use chrono::{DateTime, Local};
use crossterm::{event, ExecutableCommand};
use crossterm::event::{Event, KeyCode};
use crossterm::terminal::{disable_raw_mode, enable_raw_mode, EnterAlternateScreen, LeaveAlternateScreen};
use ratatui::{Frame, Terminal};
use ratatui::{prelude::*, widgets::*};
use ratatui::backend::CrosstermBackend;
use ratatui::layout::Flex;

struct FileData<'a> {
    path: String,
    data: Vec<Line<'a>>,
    metadata: Metadata,
    vertical_scroll: usize,
    vertical_scroll_state: ScrollbarState,
}

fn main() -> Result<(), Box<dyn Error>> {
    let args: Vec<String> = args().collect();

    let mut file_data = get_file_data(args)?;

    init_panic_hook();

    let terminal = setup_terminal()?;

    run(terminal, &mut file_data)?;

    restore_terminal()?;

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

fn run(mut terminal: Terminal<CrosstermBackend<Stdout>>, file_data: &mut FileData) -> Result<(), Box<dyn Error>> {
    let mut should_quit = false;
    while !should_quit {
        terminal.draw(|frame| ui(frame, file_data))?;
        should_quit = handle_events(file_data)?;
    }

    Ok(())
}

fn setup_terminal() -> Result<Terminal<CrosstermBackend<Stdout>>, Box<dyn Error>> {
    enable_raw_mode()?;
    stdout().execute(EnterAlternateScreen)?;
    let terminal = Terminal::new(CrosstermBackend::new(stdout()))?;

    Ok(terminal)
}

fn init_panic_hook() {
    let original_hook = take_hook();
    set_hook(Box::new(move |panic_info| {
        // intentionally ignore errors here since we're already in a panic
        let _ = restore_terminal();
        original_hook(panic_info);
    }));
}

fn restore_terminal() -> io::Result<()> {
    disable_raw_mode()?;
    stdout().execute(LeaveAlternateScreen)?;
    Ok(())
}

fn handle_events(file_data: &mut FileData) -> io::Result<bool> {
    if event::poll(Duration::from_millis(50))? {
        if let Event::Key(key) = event::read()? {
            if key.kind == event::KeyEventKind::Press {
                match key.code {
                    KeyCode::Up => {
                        file_data.vertical_scroll = file_data.vertical_scroll.saturating_sub(1);
                        file_data.vertical_scroll_state = file_data.vertical_scroll_state.position(file_data.vertical_scroll);
                    }
                    KeyCode::Down => {
                        file_data.vertical_scroll = file_data.vertical_scroll.saturating_add(1);
                        file_data.vertical_scroll_state = file_data.vertical_scroll_state.position(file_data.vertical_scroll);
                    }
                    KeyCode::Home => {
                        file_data.vertical_scroll = 0;
                        file_data.vertical_scroll_state = file_data.vertical_scroll_state.position(file_data.vertical_scroll);
                    }
                    KeyCode::End => {
                        file_data.vertical_scroll = file_data.data.len();
                        file_data.vertical_scroll_state = file_data.vertical_scroll_state.position(file_data.vertical_scroll);
                    }
                    KeyCode::Char('q') | KeyCode::Esc => {
                        return Ok(true);
                    }
                    _ => {
                        return Ok(false);
                    }
                }
            }
        }
    }

    Ok(false)
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
