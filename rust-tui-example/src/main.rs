use std::env::args;
use std::error::Error;
use std::io;
use std::io::{stdout, Stdout};
use std::panic::{set_hook, take_hook};
use std::path::Path;
use std::time::Duration;

use crossterm::{event, ExecutableCommand};
use crossterm::event::{Event, KeyCode};
use crossterm::terminal::{disable_raw_mode, enable_raw_mode, EnterAlternateScreen, LeaveAlternateScreen};
use ratatui::{Frame, Terminal};
use ratatui::{prelude::*, widgets::*};
use ratatui::backend::CrosstermBackend;

struct FileData {
    path: String,
    data: Vec<String>,
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

fn get_file_data(args: Vec<String>) -> Result<FileData, Box<dyn Error>> {
    if args.len() == 2 && !args[1].is_empty() {
        let path = args[1].clone();
        let file_path = Path::new(path.as_str());

        if file_path.exists() && file_path.is_file() {
            let contents = std::fs::read_to_string(file_path).expect("could not read file");
            println!("read {} characters", contents.len());
            let data = contents.split('\n')
                .map(|x| { x.to_string() })
                .collect::<Vec<String>>();

            Ok(FileData {
                path: file_path.display().to_string(),
                data,
            })
        } else {
            panic!("file does not exist or cannot be read")
        }
    } else {
        panic!("missing file name argument")
    }
}

fn run(mut terminal: Terminal<CrosstermBackend<Stdout>>, file_data: &mut FileData) -> Result<(), Box<dyn Error>> {
    let mut should_quit = false;
    while !should_quit {
        terminal.draw(|f| ui(f, file_data))?;
        should_quit = handle_events()?;
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

fn handle_events() -> io::Result<bool> {
    if event::poll(Duration::from_millis(50))? {
        if let Event::Key(key) = event::read()? {
            if key.kind == event::KeyEventKind::Press && key.code == KeyCode::Char('q') {
                return Ok(true);
            }
        }
    }
    Ok(false)
}

fn ui(frame: &mut Frame, file_data: &mut FileData) {
    let chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Length(3),
            Constraint::Min(1),
            Constraint::Length(3),
        ])
        .split(frame.size());

    let header_block = Block::new()
        .borders(Borders::all())
        .title_style(Style::default().fg(Color::Blue).add_modifier(Modifier::BOLD))
        .title("File Viewer");

    let file_name = ratatui::text::Line::from(file_data.path.clone())
        .style(Style::default().fg(Color::Blue));
    let header_content = Paragraph::new(file_name)
        .centered()
        .block(header_block);
    frame.render_widget(header_content, chunks[0]);

    let main_content_block = Block::new().borders(Borders::LEFT | Borders::RIGHT);

    let text : Vec<Line>= file_data.data.iter()
        .map(|line| { Line::from(line.to_string())})
        .collect();

    let main_content = Paragraph::new(text)
        .block(main_content_block)
        .wrap(Wrap { trim: false });
    frame.render_widget(main_content, chunks[1]);

    let footer_block = Block::new()
        .borders(Borders::all());

    let footer_paragraph = Paragraph::new(Text::styled("will be key commands", Style::default()))
        .centered()
        .block(footer_block);

    frame.render_widget(footer_paragraph, chunks[2]);
}
