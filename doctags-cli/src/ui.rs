use anyhow::Result;
pub use crossterm::{
    cursor,
    event::{self, Event, KeyCode, KeyEvent, KeyModifiers},
    execute, queue,
    style::{
        self, style, Attribute, Color, Colorize, ContentStyle, Print, PrintStyledContent,
        ResetColor, SetAttribute, SetBackgroundColor, SetForegroundColor, StyledContent,
    },
    terminal::{self, ClearType},
    Command,
};
use doctags::{search, Index};
use std::io::{self, Write};

pub fn ui(index: &Index) -> Result<()> {
    run(&mut io::stderr(), index)
}

fn run<W: Write>(w: &mut W, index: &Index) -> Result<()> {
    #[derive(PartialEq)]
    enum State {
        Selecting,
        Quit,
        Selected(String),
    }

    execute!(w, terminal::EnterAlternateScreen)?;

    terminal::enable_raw_mode()?;

    queue!(
        w,
        SetBackgroundColor(Color::Black),
        terminal::Clear(ClearType::All),
        style::ResetColor,
        cursor::Hide
    )?;
    let menu_normal = Color::AnsiValue(252);
    let menu_command = Color::AnsiValue(220);
    let menu_background = Color::AnsiValue(235);
    queue!(
        w,
        cursor::MoveTo(0, 0),
        SetBackgroundColor(menu_background),
        SetForegroundColor(menu_command),
        Print("ESC"),
        SetForegroundColor(menu_normal),
        Print(": quit | "),
        SetForegroundColor(menu_command),
        Print("Enter"),
        SetForegroundColor(menu_normal),
        Print(": select"),
        terminal::Clear(ClearType::UntilNewLine)
    )?;

    queue!(
        w,
        cursor::MoveTo(0, 1),
        style::ResetColor,
        SetBackgroundColor(Color::Black),
        Print("> ")
    )?;

    w.flush()?;

    let mut state = State::Selecting;
    let mut lines = Vec::new();

    // User input for search
    let mut searchinput = String::new();
    let mut selected = 0;

    let (_cols, rows) = terminal::size()?;

    while state == State::Selecting {
        if let Ok(results) = search::search_matches(index, &searchinput, (rows - 2) as usize) {
            // Ignore empty results or search errors (e.g. incomplete ':' expression)
            if results.len() > 0 {
                lines = results;
            }
        }
        queue!(w, SetBackgroundColor(Color::Black))?;
        print_selection_list(w, &lines, selected)?;
        queue!(
            w,
            cursor::MoveTo(2, 1),
            style::ResetColor,
            SetBackgroundColor(Color::Black),
            terminal::Clear(ClearType::UntilNewLine),
            Print(&searchinput),
            cursor::Show
        )?;
        w.flush()?;
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
    }

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

    terminal::disable_raw_mode()?;
    Ok(())
}

fn print_selection_list<W: Write>(
    w: &mut W,
    lines: &Vec<search::Match>,
    selected: usize,
) -> Result<()> {
    let top = 2;
    for (i, line) in lines.iter().enumerate() {
        queue!(w, cursor::MoveTo(0, top + (i as u16)))?;
        print_line(w, &line, selected == i)?;
    }
    queue!(
        w,
        cursor::MoveTo(0, top + (lines.len() as u16)),
        // Clear additional lines from previous selection
        terminal::Clear(ClearType::FromCursorDown)
    )?;
    Ok(())
}

fn print_line<W: Write>(w: &mut W, line: &search::Match, line_selected: bool) -> Result<()> {
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
            terminal::Clear(ClearType::UntilNewLine)
        )?;
    }
    Ok(())
}
