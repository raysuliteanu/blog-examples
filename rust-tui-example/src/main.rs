use std::io;
use std::io::stdout;
use std::panic::{set_hook, take_hook};
use std::time::Duration;

use crossterm::{event, ExecutableCommand};
use crossterm::event::{Event, KeyCode};
use crossterm::terminal::{disable_raw_mode, enable_raw_mode, EnterAlternateScreen, LeaveAlternateScreen};
use ratatui::{Frame, Terminal};
use ratatui::{prelude::*, widgets::*};
use ratatui::backend::CrosstermBackend;

fn main() -> io::Result<()> {
    init_panic_hook();
    enable_raw_mode()?;
    stdout().execute(EnterAlternateScreen)?;
    let mut terminal = Terminal::new(CrosstermBackend::new(stdout()))?;

    let mut should_quit = false;
    while !should_quit {
        terminal.draw(ui)?;
        should_quit = handle_events()?;
    }

    restore_terminal()?;

    Ok(())
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

fn ui(frame: &mut Frame) {
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
        .title("Will be the file name");
    frame.render_widget(header_block, chunks[0]);

    let main_content = Block::new()
        .borders(Borders::LEFT | Borders::RIGHT);
    frame.render_widget(main_content, chunks[1]);
    
    let footer_block = Block::new()
        .borders(Borders::all());

    let footer_paragraph = Paragraph::new(Text::styled("will be key commands", Style::default()))
        .centered()
        .block(footer_block);

    frame.render_widget(footer_paragraph, chunks[2]);
}
