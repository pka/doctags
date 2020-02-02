use ansi_term::{ANSIString, ANSIStrings, Colour, Style};
use anyhow::{Context, Result};
pub use crossterm::{
    cursor,
    event::{self, Event, KeyCode, KeyEvent},
    execute, queue, style,
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
        paint_selection_list(&lines, selected)?;
        if let Event::Key(KeyEvent { code, .. }) = event::read()? {
            match code {
                KeyCode::Esc => {
                    // | KeyCode::Ctrl('c')
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
                KeyCode::Char(ch) => {
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

fn paint_selection_list(lines: &Vec<search::Match>, selected: usize) -> crossterm::Result<()> {
    let mut w = io::stdout();
    let size = terminal::size()?;
    let width = size.0 as usize;
    let (_x, y) = cursor::position()?;
    for (i, line) in lines.iter().enumerate() {
        queue!(w, cursor::MoveTo(0, y + (i as u16)))?;
        w.flush()?;
        let (style, highlighted) = if selected == i {
            (Colour::White.normal(), Colour::Cyan.normal())
        } else {
            (Colour::White.dimmed(), Colour::Cyan.normal())
        };
        let mut ansi_strings = highlight(&line, style, highlighted);
        for _ in line.text.len()..width {
            ansi_strings.push(style.paint(' '.to_string()));
        }
        println!("{}", ANSIStrings(&ansi_strings));
    }
    queue!(w, cursor::MoveTo(0, y + (lines.len() as u16)))?;
    print!("{}", Colour::Blue.paint("[ESC to quit, Enter to select]"));
    w.flush()?;
    // // Clear additional lines from previous selection
    // let _ = terminal.clear(ClearType::FromCursorDown);
    Ok(())
}

fn highlight(line: &search::Match, normal: Style, highlighted: Style) -> Vec<ANSIString> {
    let mut ansi_strings = vec![];
    let snippet = &line.snippet;
    let parts = snippet.highlighted();
    if parts.len() == 0 {
        ansi_strings.push(normal.paint(&line.text));
    } else {
        let mut start_from = 0;
        for (start, end) in parts.iter().map(|h| h.bounds()) {
            ansi_strings.push(normal.paint(&snippet.fragments()[start_from..start]));
            ansi_strings.push(highlighted.paint(&snippet.fragments()[start..end]));
            start_from = end;
        }
        ansi_strings.push(normal.paint(&snippet.fragments()[start_from..]));
    }
    ansi_strings
}
