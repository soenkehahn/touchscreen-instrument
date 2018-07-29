extern crate sdl2;

use self::sdl2::event::Event;
use self::sdl2::gfx::primitives::DrawRenderer;
use self::sdl2::keyboard::Keycode;
use self::sdl2::pixels::Color;
use self::sdl2::render::Canvas;
use self::sdl2::video::Window;
use self::sdl2::EventPump;
use self::sdl2::VideoSubsystem;
use areas::{Area, Areas};
use cli;
use get_binary_name;
use ErrorString;

impl Areas {
    pub fn spawn_ui(self, cli_args: cli::Args) {
        ::std::thread::spawn(move || {
            self.run_ui(cli_args);
        });
    }

    pub fn run_ui(self, cli_args: cli::Args) {
        if let Err(e) = Ui::run_ui(cli_args, self) {
            eprintln!("error in ui thread: {:?}", e);
        }
    }
}

struct Ui {
    canvas: Canvas<Window>,
    event_pump: EventPump,
    areas: Vec<Area>,
    x_factor: f32,
    y_factor: f32,
}

impl Ui {
    fn run_ui(cli_args: cli::Args, areas: Areas) -> Result<(), ErrorString> {
        let mut ui = Ui::new(cli_args, areas)?;
        ui.run_main_loop()?;
        ui.quit();
        Ok(())
    }

    fn get_screen_rect(video_subsystem: &VideoSubsystem) -> Result<sdl2::rect::Rect, ErrorString> {
        video_subsystem
            .display_bounds(1)
            .or_else(|_| video_subsystem.display_bounds(0))
            .map_err(From::from)
    }

    fn new(cli_args: cli::Args, areas: Areas) -> Result<Ui, ErrorString> {
        let sdl_context = sdl2::init()?;
        let video_subsystem = sdl_context.video()?;
        let screen_rect = Ui::get_screen_rect(&video_subsystem)?;
        let window_size = if cli_args.dev_mode {
            (800, 600)
        } else {
            (screen_rect.width(), screen_rect.height())
        };
        let window = video_subsystem
            .window(&get_binary_name()?, window_size.0, window_size.1)
            .position(screen_rect.x(), screen_rect.y())
            .fullscreen()
            .borderless()
            .build()?;
        let canvas = window.into_canvas().build()?;
        let event_pump = sdl_context.event_pump()?;
        let mut ui = Ui {
            canvas,
            event_pump,
            areas: areas.areas.clone(),
            x_factor: window_size.0 as f32 / areas.touch_width as f32,
            y_factor: window_size.1 as f32 / areas.touch_height as f32,
        };
        ui.draw()?;
        Ok(ui)
    }

    fn run_main_loop(&mut self) -> Result<(), ErrorString> {
        'main: loop {
            match self.event_pump.wait_event() {
                Event::Quit { .. }
                | Event::KeyDown {
                    keycode: Some(Keycode::Escape),
                    ..
                } => break 'main,
                Event::Window { .. } => {
                    self.draw()?;
                }
                _ => {}
            }
        }
        Ok(())
    }

    fn quit(&self) {
        ::std::process::exit(0);
    }

    fn draw(&mut self) -> Result<(), ErrorString> {
        self.canvas.set_draw_color(Color::RGB(0, 0, 0));
        self.canvas.clear();
        for area in &self.areas {
            let (xs, ys) = &area.shape.to_polygon(self.x_factor, self.y_factor);
            let color = area.color;
            self.canvas.filled_polygon(&xs, &ys, color)?;
        }
        self.canvas.present();
        Ok(())
    }
}

impl From<self::sdl2::video::WindowBuildError> for ErrorString {
    fn from(e: self::sdl2::video::WindowBuildError) -> ErrorString {
        ErrorString(format!("{}", e))
    }
}

impl From<self::sdl2::IntegerOrSdlError> for ErrorString {
    fn from(e: self::sdl2::IntegerOrSdlError) -> ErrorString {
        ErrorString(format!("{}", e))
    }
}
