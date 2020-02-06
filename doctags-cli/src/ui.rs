use anyhow::{Context, Result};
use crossterm::{
    cursor,
    event::{self, Event, KeyCode, KeyEvent, KeyModifiers},
    execute, queue,
    style::{self, style, Color, Print, SetBackgroundColor, SetForegroundColor},
    terminal::{self, ClearType},
};
use doctags::{search, Index};
use rustyline::Editor;
use std::fs;
use std::io::{self, Write};
use std::path::Path;
use std::process::Command;

#[derive(PartialEq)]
enum CommandType {
    Foreach,
    Eachdir,
}

#[derive(PartialEq)]
enum State {
    Selecting,
    Selected(String),
    CommandExec(CommandType, String, Vec<String>),
    // ShortcutSelect,
    Keywait(Box<State>),
    Quit,
}

pub fn ui(index: &Index, outcmd: Option<String>) -> Result<()> {
    run(&mut io::stderr(), index, outcmd)
}

fn run<W: Write>(w: &mut W, index: &Index, outcmd: Option<String>) -> Result<()> {
    execute!(w, terminal::EnterAlternateScreen)?;

    terminal::enable_raw_mode()?;

    let mut state = State::Selecting;
    while state != State::Quit {
        state = match state {
            State::Selecting => select(w, index)?,
            State::CommandExec(cmdtype, command, entries) => cmdeach(w, cmdtype, command, entries)?,
            State::Selected(line) => {
                if let Some(ref fname) = outcmd {
                    fs::write(fname, format!("cd {}", line))?;
                }
                State::Quit
            }
            State::Keywait(nextstate) => keywait(w, *nextstate)?,
            _ => todo!(),
        }
    }

    execute!(
        w,
        style::ResetColor,
        cursor::Show,
        terminal::LeaveAlternateScreen
    )?;

    terminal::disable_raw_mode()?;
    Ok(())
}

fn select<W: Write>(w: &mut W, index: &Index) -> Result<State> {
    queue!(
        w,
        SetBackgroundColor(Color::Black),
        terminal::Clear(ClearType::All),
        style::ResetColor,
        cursor::Hide
    )?;

    print_menu(w)?;

    queue!(
        w,
        cursor::MoveTo(0, 1),
        style::ResetColor,
        SetBackgroundColor(Color::Black),
        Print("Search: ")
    )?;

    w.flush()?;

    let mut lines = Vec::new();

    // User input for search
    let mut searchinput = String::new();
    let mut selected = 0;

    let (_cols, rows) = terminal::size()?;

    loop {
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
            cursor::MoveTo(8, 1),
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
                    return Ok(State::Quit);
                }
                // Ctrl-c
                KeyCode::Char('c') if modifiers == KeyModifiers::CONTROL => {
                    return Ok(State::Quit);
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
                KeyCode::Char(ch) if modifiers.is_empty() || modifiers == KeyModifiers::SHIFT => {
                    searchinput.push(ch);
                    selected = 0;
                }
                KeyCode::Backspace => {
                    searchinput.pop();
                    selected = 0;
                }
                // Alt-o
                KeyCode::Char('o') if modifiers == KeyModifiers::ALT => {
                    let _ = open::that(&lines[selected].text);
                }
                // Alt-p
                KeyCode::Char('p') if modifiers == KeyModifiers::ALT => {
                    if let Ok(dir) = entry_dir(&lines[selected].text) {
                        let _ = open::that(&dir);
                    }
                    // ignore errors
                }
                // Alt-c
                KeyCode::Char('c') if modifiers == KeyModifiers::ALT => {
                    if let Ok(dir) = entry_dir(&lines[selected].text) {
                        if let Some(dirstr) = dir.to_str() {
                            return Ok(State::Selected(dirstr.to_string()));
                        }
                    }
                    // ignore errors
                }
                // Alt-f
                KeyCode::Char('f') if modifiers == KeyModifiers::ALT => {
                    let entries: Vec<String> = lines.iter().map(|line| line.text.clone()).collect();
                    return Ok(enter_shell_command(w, CommandType::Foreach, entries)?);
                }
                // Alt-d
                KeyCode::Char('d') if modifiers == KeyModifiers::ALT => {
                    let entries: Vec<String> = lines.iter().map(|line| line.text.clone()).collect();
                    return Ok(enter_shell_command(w, CommandType::Eachdir, entries)?);
                }
                _ => {
                    // println!("{}", format!("OTHER InputEvent: {:?}\n\n", code));
                }
            }
        }
    }
}

fn entry_dir(fname: &str) -> Result<&Path> {
    let p = Path::new(fname);
    let dir = if p.is_dir() {
        p
    } else {
        p.parent().context("invalid parent")?
    };
    Ok(dir)
}

fn enter_shell_command<W: Write>(
    w: &mut W,
    cmdtype: CommandType,
    entries: Vec<String>,
) -> Result<State> {
    queue!(
        w,
        cursor::MoveTo(0, 1),
        style::ResetColor,
        SetBackgroundColor(Color::Black),
    )?;
    w.flush()?;

    let mut rl = Editor::<()>::new();
    let _ok = rl.load_history("/tmp/history.txt");
    let readline = rl.readline("Command: ");
    let state = match readline {
        Ok(line) => {
            if line.is_empty() {
                enter_shell_command(w, cmdtype, entries)
            } else {
                rl.add_history_entry(line.as_str());
                Ok(State::CommandExec(cmdtype, line, entries))
            }
        }
        Err(_) => Ok(State::Selecting),
    };
    let _ok = rl.save_history("/tmp/history.txt");
    state
}

fn cmdeach<W: Write>(
    w: &mut W,
    cmdtype: CommandType,
    command: String,
    entries: Vec<String>,
) -> Result<State> {
    queue!(
        w,
        cursor::MoveTo(0, 2),
        style::ResetColor,
        SetBackgroundColor(Color::Black),
        terminal::Clear(ClearType::FromCursorDown)
    )?;
    w.flush()?;

    execute!(w, terminal::LeaveAlternateScreen)?;
    terminal::disable_raw_mode()?;

    let mut args: Vec<&str> = command.split(' ').collect();
    args.retain(|&arg| !arg.is_empty());
    let cmd = args.remove(0);

    for entry in entries {
        match cmdtype {
            CommandType::Foreach => {
                println!("\n{} {}", &command, style(&entry).with(Color::Yellow));
                if let Err(status) = Command::new(cmd).args(&args).arg(entry).status() {
                    println!("{}", &status);
                }
            }
            CommandType::Eachdir => {
                if Path::new(&entry).is_dir() {
                    println!("\ncd {} && {}", style(&entry).with(Color::Yellow), &command);
                    if let Err(status) = Command::new(cmd).args(&args).current_dir(&entry).status()
                    {
                        println!("{}", &status);
                    }
                } else {
                    println!("\nSkipping {}", style(&entry).with(Color::Yellow));
                }
            }
        }
    }

    // Reenabling raw_mode and switching back to AlternateScreen in keywait
    Ok(State::Keywait(Box::new(State::Selecting)))
}

fn keywait<W: Write>(w: &mut W, next_state: State) -> Result<State> {
    println!("\nPress [Esc] to exit or any other key to return...");
    terminal::enable_raw_mode()?;
    let mut next = next_state;
    if let Event::Key(KeyEvent { code, modifiers }) = event::read()? {
        next = match code {
            KeyCode::Esc => State::Quit,
            // Ctrl-c
            KeyCode::Char('c') if modifiers == KeyModifiers::CONTROL => State::Quit,
            _ => next,
        };
    }
    execute!(w, terminal::EnterAlternateScreen)?;
    Ok(next)
}

fn print_menu<W: Write>(w: &mut W) -> Result<()> {
    let entries = [
        ("ESC", "quit"),
        // ("Enter", "select"),
        // ("Alt-v", "view"),
        ("Alt-o", "open"),
        ("Alt-p", "open folder"),
        ("Alt-c", "cd"),
        ("Alt-f", "foreach"),
        ("Alt-d", "eachdir"),
        // ("Alt-s", "shortcut"),
        // ("Alt-c", "edit config"),
    ];
    let menu_normal = Color::AnsiValue(252);
    let menu_command = Color::AnsiValue(220);
    let menu_background = Color::AnsiValue(235);
    queue!(w, cursor::MoveTo(0, 0), SetBackgroundColor(menu_background))?;
    for (i, (cmd, desc)) in entries.iter().enumerate() {
        queue!(
            w,
            SetForegroundColor(menu_command),
            Print(cmd),
            SetForegroundColor(menu_normal),
            Print(": "),
            Print(desc),
        )?;
        if i < entries.len() - 1 {
            queue!(w, Print(" | "),)?;
        }
    }
    queue!(w, terminal::Clear(ClearType::UntilNewLine))?;
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
