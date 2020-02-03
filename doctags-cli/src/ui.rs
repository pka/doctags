use anyhow::{Context, Result};
pub use crossterm::{
    cursor,
    event::{self, Event, KeyCode, KeyEvent, KeyModifiers},
    execute, queue,
    style::{
        self, style, Attribute, Color, Colorize, ContentStyle, Print, PrintStyledContent,
        ResetColor, SetAttribute, SetForegroundColor, StyledContent,
    },
    terminal::{self, ClearType},
    Command,
};
use doctags::{search, Index};
use std::io::{self, Write};

pub fn ui(index: &Index) -> Result<()> {
    let mut stderr = io::stdout();
    run(&mut stderr, index).with_context(|| format!("crossterm result"))
}

fn run<W>(w: &mut W, index: &Index) -> crossterm::Result<()>
where
    W: Write,
{
    #[derive(PartialEq)]
    enum State {
        Selecting,
        Quit,
        Selected(String),
    }
    let mut state = State::Selecting;
    let mut lines = Vec::new();

    execute!(w, terminal::EnterAlternateScreen)?;

    terminal::enable_raw_mode()?;

    // User input for search
    let mut searchinput = String::new();
    let mut selected = 0;

    let (_cols, rows) = terminal::size()?;
    queue!(
        w,
        style::ResetColor,
        terminal::Clear(ClearType::All),
        cursor::Hide,
        cursor::MoveTo(1, 1)
    )?;
    w.flush()?;

    while state == State::Selecting {
        if let Ok(results) = search::search_matches(index, &searchinput, (rows - 1) as usize) {
            // Ignore empty results or search errors (e.g. incomplete ':' expression)
            if results.len() > 0 {
                lines = results;
            }
        }
        print_selection_list(&lines, selected)?;
        if let Event::Key(KeyEvent { code, modifiers }) = event::read()? {
            match code {
                KeyCode::Esc => {
                    state = State::Quit;
                }
                // Ctrl-c
                KeyCode::Char('c') if modifiers == KeyModifiers::CONTROL => {
                    state = State::Quit;
                }
                KeyCode::Up => {
                    if selected > 0 {
                        selected -= 1;
                    }
                }
                KeyCode::Down => {
                    if selected + 1 < lines.len() {
                        selected += 1;
                    }
                }
                KeyCode::Enter => {
                    state = State::Selected(lines[selected].text.clone());
                }
                KeyCode::Char(ch) if modifiers.is_empty() || modifiers == KeyModifiers::SHIFT => {
                    searchinput.push(ch);
                    selected = 0;
                }
                KeyCode::Backspace => {
                    searchinput.pop();
                    selected = 0;
                }
                _ => {
                    // println!("{}", format!("OTHER InputEvent: {:?}\n\n", code));
                }
            }
        }
        //         cursor.move_up(lines.len() as u16);
    }
    //     let (_x, y) = cursor.pos();
    //     cursor.goto(0, y)?;
    //     let _ = cursor.show();

    // let _ = terminal.clear(ClearType::FromCursorDown);

    execute!(
        w,
        style::ResetColor,
        cursor::Show,
        terminal::LeaveAlternateScreen
    )?;

    match state {
        State::Selected(line) => {
            print!("{}", line);
        }
        _ => {}
    }

    terminal::disable_raw_mode()
}

fn print_selection_list(lines: &Vec<search::Match>, selected: usize) -> crossterm::Result<()> {
    let mut w = io::stdout();
    let size = terminal::size()?;
    let width = size.0 as usize;
    // let (_x, y) = cursor::position()?;
    let y = 0;
    for (i, line) in lines.iter().enumerate() {
        queue!(w, cursor::MoveTo(0, y + (i as u16)))?;
        print_line(&line, selected == i)?;
        for _ in line.text.len()..width {
            queue!(w, Print(" "))?;
        }
    }
    queue!(w, cursor::MoveTo(0, y + (lines.len() as u16)))?;
    queue!(
        w,
        PrintStyledContent("[ESC to quit, Enter to select]".blue()),
        // Clear additional lines from previous selection
        terminal::Clear(ClearType::FromCursorDown)
    )?;
    w.flush()?;
    Ok(())
}

fn print_line(line: &search::Match, line_selected: bool) -> crossterm::Result<()> {
    let mut w = io::stdout();
    let line_color = if line_selected {
        Color::White
    } else {
        Color::Grey
    };
    let highlight_color = Color::Cyan;
    let snippet = &line.snippet;
    let parts = snippet.highlighted();
    if parts.len() == 0 {
        queue!(w, SetForegroundColor(line_color), Print(&line.text),)?;
    } else {
        let mut start_from = 0;
        for (start, end) in parts.iter().map(|h| h.bounds()) {
            queue!(
                w,
                // Normal
                SetForegroundColor(line_color),
                Print(&snippet.fragments()[start_from..start]),
                // Highlighted
                SetForegroundColor(highlight_color),
                Print(&snippet.fragments()[start..end])
            )?;
            start_from = end;
        }
        queue!(
            w,
            SetForegroundColor(line_color),
            Print(&snippet.fragments()[start_from..]),
        )?;
    }
    Ok(())
}
