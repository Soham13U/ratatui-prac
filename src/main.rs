use color_eyre::{Result, eyre::WrapErr};
use crossterm::event::{self, Event, KeyCode, KeyEvent, KeyEventKind};
use ratatui::widgets::{Bar, BarChart};
use ratatui::{
    DefaultTerminal, Frame,
    buffer::Buffer,
    layout::{Constraint, Layout, Rect},
    style::{Color, Style, Stylize},
    symbols::border,
    text::{Line, Text},
    widgets::{Block, BorderType, Gauge, Paragraph, Widget},
};
use std::sync::mpsc;
use std::time::Duration;
use std::{io, thread};
fn main() -> io::Result<()> {
    let mut terminal = ratatui::init();
    let mut app = App {
        exit: false,
        progress_bar_color: Color::Cyan,
        color_vec: vec![
            Color::Cyan,
            Color::Red,
            Color::Green,
            Color::LightRed,
            Color::Yellow,
        ],
        background_progress: 0_f64,
    };
    let (event_tx, event_rx) = mpsc::channel::<CEvent>();
    let app_result = app.run(&mut terminal, event_rx);
    let tx_to_input_events = event_tx.clone();
    thread::spawn(move || {
        handle_input_events(tx_to_input_events);
    });
    let tx_to_background = event_tx.clone();
    thread::spawn(move || {
        run_background_thread(tx_to_background);
    });

    ratatui::restore();
    app_result
}
enum CEvent {
    Input(crossterm::event::KeyEvent),
    Progress(f64),
}
fn handle_input_events(tx: mpsc::Sender<CEvent>) {
    loop {
        match crossterm::event::read().unwrap() {
            crossterm::event::Event::Key(key_event) => tx.send(CEvent::Input(key_event)).unwrap(),
            _ => {}
        }
    }
}
fn run_background_thread(tx: mpsc::Sender<CEvent>) {
    let mut progress = 0_f64;
    let increment = 0.01_f64;
    loop {
        thread::sleep(Duration::from_millis(100));
        progress += increment;
        progress = progress.min(1_f64);
        tx.send(CEvent::Progress(progress)).unwrap();
    }
}
pub struct App {
    exit: bool,
    progress_bar_color: Color,
    color_vec: Vec<Color>,
    background_progress: f64,
}
impl App {
    fn run(
        &mut self,
        terminal: &mut DefaultTerminal,
        rx: mpsc::Receiver<CEvent>,
    ) -> io::Result<()> {
        while !self.exit {
            match rx.recv().unwrap() {
                CEvent::Input(key_event) => self.handle_key_event(key_event)?,
                CEvent::Progress(progress) => self.background_progress = progress,
            }
            terminal.draw(|frame| self.draw(frame))?;
            self.handle_events().wrap_err("Handle events faield");
        }

        Ok(())
    }
    fn draw(&self, frame: &mut Frame) {
        frame.render_widget(self, frame.area());
    }
    fn handle_events(&mut self) -> Result<()> {
        match event::read()? {
            crossterm::event::Event::Key(key_event) if key_event.kind == KeyEventKind::Press => {
                self.handle_key_event(key_event)
                    .wrap_err_with(|| format!("handling failed"))
            }
            _ => Ok(()),
        }
    }

    fn handle_key_event(&mut self, key_event: crossterm::event::KeyEvent) -> io::Result<()> {
        match key_event.code {
            KeyCode::Char('q') => self.exit(),
            KeyCode::Char('n') => self.change_color('n'),
            KeyCode::Char('p') => self.change_color('p'),
            _ => {}
        }
        Ok(())
    }
    fn exit(&mut self) {
        self.exit = true;
    }
    fn change_color(&mut self, input: char) {
        //self.progress_bar_color = Color::Red;
        // let n: usize = rand::thread_rng().gen_range(0..3);
        let curr_color = self.progress_bar_color;
        let curr_color_index: usize = self
            .color_vec
            .iter()
            .position(|&x| x == curr_color)
            .expect("value not found");
        let index: usize = match input {
            'n' => {
                if curr_color_index == self.color_vec.len() - 1 {
                    0
                } else {
                    curr_color_index + 1
                }
            }
            'p' => {
                if curr_color_index == 0 {
                    self.color_vec.len() - 1
                } else {
                    curr_color_index - 1
                }
            }
            _ => 0,
        };
        self.progress_bar_color = self.color_vec[index];
    }
}
impl Widget for &App {
    fn render(self, area: Rect, buf: &mut Buffer) {
        let vertical_layout = Layout::vertical([
            Constraint::Percentage(20),
            Constraint::Percentage(30),
            Constraint::Percentage(50),
        ]);
        let horizontal_layout =
            Layout::horizontal([Constraint::Percentage(50), Constraint::Percentage(50)]);
        let [title_area, gauge_area, graph_area] = vertical_layout.areas(area);
        let [a1, a2] = graph_area.layout(&horizontal_layout);

        let bars = vec![
            Bar::with_label("Red", 30).red(),
            Bar::with_label("Blue", 20).blue(),
            Bar::with_label("Green", 15).green(),
        ];
        let chart = BarChart::horizontal(bars).bar_width(1);

        let def_block = Block::bordered()
            .border_type(BorderType::LightDoubleDashed)
            .yellow();

        chart.block(def_block).render(a1, buf);
        let text = "This is some text\nwith multiple lines.\nHello there!";
        Paragraph::new(text)
            .bold()
            .centered()
            .white()
            .block(
                Block::bordered()
                    .border_type(BorderType::QuadrantInside)
                    .light_green(),
            )
            .render(a2, buf);
        Line::from("test line")
            .bold()
            .left_aligned()
            .red()
            .render(title_area, buf);
        Line::from("test line 2")
            .bold()
            .right_aligned()
            .red()
            .render(title_area, buf);
        Line::from("Hello World")
            .bold()
            .centered()
            .render(title_area, buf);
        let instructions = Line::from(vec![
            "Quit".into(),
            "<Q>".blue().bold(),
            "PrevColor".into(),
            "<P>".blue().bold(),
            "NextColor".into(),
            "<N>".blue().bold(),
        ])
        .centered();
        let block = Block::bordered()
            .title(Line::from("Processes"))
            .title_bottom(instructions)
            .border_set(border::THICK);
        let progress_bar = Gauge::default()
            .gauge_style(Style::default().fg(self.progress_bar_color))
            .block(block)
            .label(format!(
                "Progress meter: {:.2}%",
                self.background_progress * 100_f64
            ))
            .ratio(self.background_progress);
        progress_bar.render(
            Rect {
                x: gauge_area.left(),
                y: gauge_area.top(),
                width: gauge_area.width,
                height: 3,
            },
            buf,
        );
    }
}
