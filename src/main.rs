mod spotify;
use crossterm::{
    cursor::{Hide, MoveTo, Show},
    event::{
        self, DisableMouseCapture, EnableMouseCapture, Event, KeyCode, KeyEventKind, KeyModifiers,
        MouseButton, MouseEventKind,
    },
    execute, queue,
    style::{Color, Print, ResetColor, SetForegroundColor},
    terminal::{self, Clear, ClearType, EnterAlternateScreen, LeaveAlternateScreen},
};
use std::{
    io::{self, IsTerminal, Write},
    time::{Duration, Instant},
};

use unicode_width::UnicodeWidthChar;

const HELP: &str = "omegatimer [--loop] [--glyph CHAR] [STUDY:REST ...]\n\nDurations are whole minutes. Run without arguments for clickable setup.\n--glyph sets the clock fill to one visible, single-column character (default █).\nExample: omegatimer --glyph '#' --loop 25:5 50:10";
type Pair = [u32; 2];

fn parse_pair(arg: &str) -> Option<Pair> {
    fn minutes(s: &str) -> Option<u32> {
        if !s.bytes().all(|b| b.is_ascii_digit()) {
            return None;
        }
        s.parse().ok().filter(|&n| n > 0)
    }
    let (a, b) = arg.split_once(':')?;
    Some([minutes(a)?, minutes(b)?])
}

struct Timer {
    chain: Vec<Pair>,
    index: usize,
    phase: usize,
    elapsed: Duration,
    paused: bool,
    repeat: bool,
}

impl Timer {
    fn length(&self) -> Option<Duration> {
        self.chain
            .get(self.index)
            .map(|p| Duration::from_secs(u64::from(p[self.phase]) * 60))
    }
    fn tick(&mut self, delta: Duration) {
        if self.paused || self.length().is_none() {
            return;
        }
        self.elapsed += delta;
        while let Some(length) = self.length() {
            if self.elapsed < length {
                return;
            }
            self.elapsed -= length;
            self.phase ^= 1;
            if self.phase == 0 {
                self.index += 1;
                if self.repeat && self.index == self.chain.len() {
                    self.index = 0;
                }
            }
        }
        self.elapsed = Duration::ZERO;
    }
}

#[derive(Clone, Copy)]
enum Action {
    Adjust(usize, bool),
    Add,
    Loop,
    Pause,
    Quit,
}

const MINT: Color = Color::Rgb {
    r: 135,
    g: 218,
    b: 181,
};
const LILAC: Color = Color::Rgb {
    r: 190,
    g: 164,
    b: 236,
};
const MUTED: Color = Color::Rgb {
    r: 145,
    g: 150,
    b: 165,
};

struct Frame {
    bytes: Vec<u8>,
    buttons: Vec<(u16, u16, u16, Action)>,
    x: u16,
    y: u16,
}

impl Frame {
    fn label(&mut self, x: u16, y: u16, color: Color, s: impl std::fmt::Display) -> io::Result<()> {
        queue!(
            self.bytes,
            MoveTo(self.x + x, self.y + y),
            SetForegroundColor(color),
            Print(s)
        )
    }
    fn button(&mut self, x: u16, y: u16, s: &str, a: Action) -> io::Result<()> {
        self.buttons
            .push((self.x + x, self.x + x + s.len() as u16, self.y + y, a));
        self.label(x, y, Color::Reset, s)
    }
}

// Three pixels across by five high, with a narrower colon.
const FONT: [[u8; 5]; 11] = [
    [7, 5, 5, 5, 7],
    [2, 6, 2, 2, 7],
    [7, 1, 7, 4, 7],
    [7, 1, 7, 1, 7],
    [5, 5, 7, 1, 1],
    [7, 4, 7, 1, 7],
    [7, 4, 7, 5, 7],
    [7, 1, 1, 1, 1],
    [7, 5, 7, 5, 7],
    [7, 5, 7, 1, 7],
    [0, 1, 0, 1, 0],
];

fn clock_rows(clock: &str, glyph: char, scale: usize) -> [String; 5] {
    std::array::from_fn(|row| {
        let mut line = String::new();
        for byte in clock.bytes() {
            if !line.is_empty() {
                line.push(' ');
            }
            let (index, bits) = if byte == b':' {
                (10, 1)
            } else {
                ((byte - b'0') as usize, 3)
            };
            for bit in (0..bits).rev() {
                let fill = if FONT[index][row] & (1 << bit) != 0 {
                    glyph
                } else {
                    ' '
                };
                line.extend(std::iter::repeat_n(fill, scale));
            }
        }
        line
    })
}

fn parse_glyph(value: &str) -> Option<char> {
    let mut chars = value.chars();
    let c = chars.next()?;
    (chars.next().is_none() && !c.is_whitespace() && c.width() == Some(1)).then_some(c)
}

// A terminal row is usually about twice as tall as a column is wide.
fn square_tile(w: u16, h: u16, aspect: f64) -> (u16, u16) {
    let width = (f64::from(h) * aspect).round() as u16;
    if width <= w {
        (width, h)
    } else {
        (w, (f64::from(w) / aspect).round() as u16)
    }
}

fn draw_cat(f: &mut Frame, x: u16, y: u16, w: u16, h: u16, beat: usize) -> io::Result<()> {
    const POSES: [[&str; 5]; 4] = [
        [
            r"   /\_/\   ",
            r"  ( o.o )  ",
            r" \(     )_ ",
            r"   )   (   ",
            r"  /_/ \_\  ",
        ],
        [
            r"    /\_/\  ",
            r"   ( ^.^ ) ",
            r"  _(     )/",
            r"    )   (  ",
            r"   /_/  )_ ",
        ],
        [
            r"   /\_/\   ",
            r"  ( o.o )  ",
            r" _(     )/ ",
            r"   )   (   ",
            r"  /_/ \_\  ",
        ],
        [
            r"  /\_/\    ",
            r" ( ^.^ )   ",
            r"\(     )_  ",
            r"  )   (    ",
            r" _(  \_\   ",
        ],
    ];
    let lines: &[&str] = if w >= 11 && h >= 5 {
        &POSES[beat % 4]
    } else if w >= 5 && h >= 1 {
        if beat.is_multiple_of(2) {
            &[r"=^.^="]
        } else {
            &[r"=^o^="]
        }
    } else {
        return Ok(());
    };
    let sw = lines[0].len() as u16;
    let sh = lines.len() as u16;
    for (i, line) in lines.iter().enumerate() {
        f.label(x + (w - sw) / 2, y + (h - sh) / 2 + i as u16, LILAC, line)?;
    }
    Ok(())
}

fn draw(
    timer: &Timer,
    draft: Pair,
    glyph: char,
    (w, h): (u16, u16),
    aspect: f64,
    beat: usize,
    playback: &spotify::Playback,
) -> io::Result<Frame> {
    let (tile_w, tile_h) = square_tile(w, h, aspect);
    let mut f = Frame {
        bytes: Vec::new(),
        buttons: Vec::new(),
        x: 0,
        y: 0,
    };
    queue!(f.bytes, Clear(ClearType::All))?;
    if tile_w < w {
        draw_cat(&mut f, tile_w, 0, w - tile_w, h, beat)?;
    } else if tile_h < h {
        draw_cat(&mut f, 0, tile_h, w, h - tile_h, beat)?;
    }
    if tile_w < 22 || tile_h < 11 {
        f.label(
            0,
            0,
            MUTED,
            "Enlarge timer tile"
                .chars()
                .take(tile_w.into())
                .collect::<String>(),
        )?;
        return Ok(f);
    }
    let inset = if tile_w >= 44 { 2 } else { 0 };
    f.x = inset;
    let width = tile_w - 2 * inset;
    let compact = width < 42 || tile_h < 19;
    f.label(0, 0, MINT, "Ω  omegatimer")?;
    f.button(width - 6, 0, "[Quit]", Action::Quit)?;
    if !compact {
        f.label(0, 1, MUTED, "─".repeat(width as usize))?;
    }
    let len = timer.length();
    let accent = if timer.phase == 0 { MINT } else { LILAC };
    let status = match len {
        Some(_) => format!(
            "{}   ·   {}/{}{}",
            ["Deep study", "Rest"][timer.phase],
            timer.index + 1,
            timer.chain.len(),
            if timer.paused { "   ·   paused" } else { "" }
        ),
        None if timer.chain.is_empty() => "Set your first session".into(),
        None => "Chain complete".into(),
    };
    f.label(
        0,
        if compact { 1 } else { 2 },
        accent,
        status.chars().take(width.into()).collect::<String>(),
    )?;
    let remaining = len
        .map(|d| d.as_secs() - timer.elapsed.as_secs())
        .unwrap_or(u64::from(draft[0]) * 60);
    let clock = if len.is_none() && !timer.chain.is_empty() {
        "00:00".into()
    } else {
        format!("{:02}:{:02}", remaining / 60, remaining % 60)
    };
    let pixels = (clock.len() - 1) * 3 + 1;
    let gaps = clock.len() - 1;
    let bar_y = tile_h - if compact { 6 } else { 11 };
    let clock_top = if compact { 2 } else { 3 };
    let available = bar_y - clock_top;
    let max_scale = (usize::from(width) - gaps) / pixels;
    let scale_y = (available / 5).min(((max_scale as f64 / aspect).round() as u16).max(1));
    if scale_y == 0 || max_scale == 0 {
        f.label(
            (width - clock.len() as u16) / 2,
            clock_top + available / 2,
            accent,
            clock,
        )?;
    } else {
        let scale_x = max_scale.min((f64::from(scale_y) * aspect).round().max(1.0) as usize);
        let left = (width - (pixels * scale_x + gaps) as u16) / 2;
        let top = clock_top + (available - 5 * scale_y) / 2;
        for (i, line) in clock_rows(&clock, glyph, scale_x).iter().enumerate() {
            for dy in 0..scale_y {
                f.label(left, top + i as u16 * scale_y + dy, accent, line)?;
            }
        }
    }
    let pct = match len {
        Some(length) => (timer.elapsed.as_millis() * 100 / length.as_millis()) as usize,
        None if !timer.chain.is_empty() => 100,
        None => 0,
    };
    let cells = width as usize - 6;
    let filled = cells * pct / 100;
    f.label(0, bar_y, accent, "━".repeat(filled))?;
    f.label(
        filled as u16,
        bar_y,
        MUTED,
        format!("{}  {:3}%", "─".repeat(cells - filled), pct),
    )?;
    if compact {
        for (i, name) in ["Study", "Rest"].iter().enumerate() {
            let row = bar_y + 1 + i as u16;
            f.label(0, row, MUTED, name)?;
            f.label(6, row, Color::Reset, draft[i])?;
            f.button(width - 6, row, "[-]", Action::Adjust(i, false))?;
            f.button(width - 3, row, "[+]", Action::Adjust(i, true))?;
        }
        f.button(
            0,
            bar_y + 3,
            if len.is_some() { "[Queue]" } else { "[Start]" },
            Action::Add,
        )?;
        f.button(
            8,
            bar_y + 3,
            if timer.repeat {
                "[Loop:on]"
            } else {
                "[Loop:off]"
            },
            Action::Loop,
        )?;
        if len.is_some() {
            f.button(
                0,
                bar_y + 4,
                if timer.paused { "[Resume]" } else { "[Pause]" },
                Action::Pause,
            )?;
        }
        f.label(0, bar_y + 5, MUTED, playback.row(width))?;
        return Ok(f);
    }
    f.y = tile_h - 19;
    f.label(0, 9, MUTED, "SESSION LENGTH  /  MINUTES")?;
    for (i, name) in ["Study", "Rest"].iter().enumerate() {
        let x = i as u16 * 21;
        f.label(x, 10, MUTED, name)?;
        f.button(x, 11, "[-]", Action::Adjust(i, false))?;
        f.label(x + 4, 11, Color::Reset, draft[i])?;
        f.button(x + 15, 11, "[+]", Action::Adjust(i, true))?;
    }
    f.button(
        0,
        13,
        if len.is_some() {
            "[+ Queue]"
        } else {
            "[Start]"
        },
        Action::Add,
    )?;
    f.button(
        12,
        13,
        if timer.repeat {
            "[Loop: on]"
        } else {
            "[Loop: off]"
        },
        Action::Loop,
    )?;
    if len.is_some() {
        f.button(
            26,
            13,
            if timer.paused { "[Resume]" } else { "[Pause]" },
            Action::Pause,
        )?;
    }
    f.label(0, 15, MUTED, "─".repeat(width as usize))?;
    f.label(0, 16, MUTED, playback.row(width))?;
    f.label(0, 18, MUTED, "a/z study  s/x rest  ↵ add  L loop  ␣ hold")?;
    Ok(f)
}

struct Screen;
impl Drop for Screen {
    fn drop(&mut self) {
        let _ = execute!(
            io::stdout(),
            ResetColor,
            DisableMouseCapture,
            Show,
            LeaveAlternateScreen
        );
        let _ = terminal::disable_raw_mode();
    }
}

fn run(mut timer: Timer, glyph: char) -> io::Result<()> {
    let mut out = io::stdout();
    if !out.is_terminal() || !io::stdin().is_terminal() {
        return Err(io::Error::other(
            "open omegatimer in an interactive terminal",
        ));
    }
    terminal::enable_raw_mode()?;
    let _screen = Screen;
    execute!(out, EnterAlternateScreen, Hide, EnableMouseCapture)?;
    let mut draft = timer.chain.last().copied().unwrap_or([25, 5]);
    let music = spotify::watch();
    let mut playback = spotify::Playback::default();
    let mut previous = Vec::new();
    let animation_start = Instant::now();
    let mut last = animation_start;
    loop {
        let now = Instant::now();
        timer.tick(now - last);
        last = now;
        let (size, aspect) = match terminal::window_size() {
            Ok(s) if s.columns > 0 && s.rows > 0 => {
                let ratio = if s.width > 0 && s.height > 0 {
                    f64::from(s.height) * f64::from(s.columns)
                        / (f64::from(s.width) * f64::from(s.rows))
                } else {
                    2.0
                };
                ((s.columns, s.rows), ratio)
            }
            _ => (terminal::size()?, 2.0),
        };
        if let Some(latest) = music.try_iter().last() {
            playback = latest;
        }
        let f = draw(
            &timer,
            draft,
            glyph,
            size,
            aspect,
            (animation_start.elapsed().as_millis() / 320) as usize,
            &playback,
        )?;
        if previous != f.bytes {
            out.write_all(&f.bytes)?;
            out.flush()?;
            previous = f.bytes;
        }
        if !event::poll(Duration::from_millis(100))? {
            continue;
        }
        let action = match event::read()? {
            Event::Mouse(m) if m.kind == MouseEventKind::Down(MouseButton::Left) => f
                .buttons
                .iter()
                .find(|(left, right, y, _)| (*left..*right).contains(&m.column) && *y == m.row)
                .map(|b| b.3),
            Event::Key(k) if k.kind == KeyEventKind::Press => match k.code {
                KeyCode::Char('q') | KeyCode::Esc => Some(Action::Quit),
                KeyCode::Char('c') if k.modifiers.contains(KeyModifiers::CONTROL) => {
                    Some(Action::Quit)
                }
                KeyCode::Char('a') => Some(Action::Adjust(0, true)),
                KeyCode::Char('z') => Some(Action::Adjust(0, false)),
                KeyCode::Char('s') => Some(Action::Adjust(1, true)),
                KeyCode::Char('x') => Some(Action::Adjust(1, false)),
                KeyCode::Enter => Some(Action::Add),
                KeyCode::Char('l' | 'L') => Some(Action::Loop),
                KeyCode::Char(' ') => Some(Action::Pause),
                _ => None,
            },
            _ => None,
        };
        let now = Instant::now();
        timer.tick(now - last);
        last = now;
        match action {
            Some(Action::Quit) => return Ok(()),
            Some(Action::Adjust(i, up)) => {
                draft[i] = if up {
                    draft[i].saturating_add(1)
                } else {
                    draft[i].saturating_sub(1).max(1)
                }
            }
            Some(Action::Loop) => timer.repeat = !timer.repeat,
            Some(Action::Pause) if timer.length().is_some() => timer.paused = !timer.paused,
            Some(Action::Add) => {
                if timer.length().is_none() {
                    timer.chain.clear();
                    timer.index = 0;
                }
                timer.chain.push(draft);
            }
            _ => {}
        }
    }
}

fn main() -> std::process::ExitCode {
    let mut timer = Timer {
        chain: vec![],
        index: 0,
        phase: 0,
        elapsed: Duration::ZERO,
        paused: false,
        repeat: false,
    };
    let mut glyph = '█';
    let mut args = std::env::args().skip(1);
    while let Some(arg) = args.next() {
        match arg.as_str() {
            "--help" | "-h" => {
                println!("{HELP}");
                return std::process::ExitCode::SUCCESS;
            }
            "--loop" => timer.repeat = true,
            "--glyph" => match args.next().as_deref().and_then(parse_glyph) {
                Some(c) => glyph = c,
                None => {
                    eprintln!(
                        "--glyph needs one visible, single-column character, e.g. --glyph '#'."
                    );
                    return 2.into();
                }
            },
            _ => match parse_pair(&arg) {
                Some(pair) => timer.chain.push(pair),
                None => {
                    eprintln!("Invalid session {arg:?}. Use positive whole minutes, e.g. 25:5.");
                    return 2.into();
                }
            },
        }
    }
    match run(timer, glyph) {
        Ok(()) => std::process::ExitCode::SUCCESS,
        Err(e) => {
            eprintln!("omegatimer: {e}");
            std::process::ExitCode::FAILURE
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn custom_clock_pixels() {
        let rows = clock_rows("0:2", '#', 1);
        assert_eq!(
            rows,
            [
                "###   ###",
                "# # #   #",
                "# #   ###",
                "# # # #  ",
                "###   ###"
            ]
        );
        assert_eq!(parse_glyph("Ω"), Some('Ω'));
        for invalid in ["", "##", " ", "界", "\x1b", "\u{0301}"] {
            assert_eq!(parse_glyph(invalid), None);
        }
    }
    #[test]
    fn cli_pairs() {
        assert_eq!(parse_pair("50:10"), Some([50, 10]));
        for invalid in ["25", "2:0", "-1:5", "2:3:4", "99999999999:1"] {
            assert_eq!(parse_pair(invalid), None);
        }
    }
    #[test]
    fn chained_clock() {
        let mut t = Timer {
            chain: vec![[1, 2], [3, 4]],
            index: 0,
            phase: 0,
            elapsed: Duration::ZERO,
            paused: true,
            repeat: true,
        };
        t.tick(Duration::from_secs(60));
        assert_eq!(t.elapsed, Duration::ZERO);
        t.paused = false;
        t.tick(Duration::from_secs(65));
        assert_eq!((t.index, t.phase, t.elapsed.as_secs()), (0, 1, 5));
        t.tick(Duration::from_secs(115));
        assert_eq!((t.index, t.phase, t.elapsed.as_secs()), (1, 0, 0));
        t.tick(Duration::from_secs(421));
        assert_eq!((t.index, t.phase, t.elapsed.as_secs()), (0, 0, 1));
        t.repeat = false;
        t.tick(Duration::from_secs(600));
        assert_eq!(t.length(), None);
    }
}
