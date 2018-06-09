extern crate sdl2;

use self::sdl2::event::Event;
use self::sdl2::keyboard::Keycode;
use self::sdl2::pixels::Color;
use self::sdl2::render::Canvas;
use self::sdl2::video::Window;
use self::sdl2::EventPump;
use self::sdl2::VideoSubsystem;
use areas::Areas;
use get_binary_name;
use ErrorString;

impl Areas {
    pub fn spawn_ui(self) {
        ::std::thread::spawn(move || {
            if let Err(e) = Ui::run_ui(self) {
                eprintln!("error in ui thread: {:?}", e);
            }
        });
    }
}

struct Ui {
    canvas: Canvas<Window>,
    event_pump: EventPump,
    ui_elements: Vec<(sdl2::rect::Rect, Color)>,
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

impl Ui {
    fn run_ui(areas: Areas) -> Result<(), ErrorString> {
        let mut ui = Ui::new(areas)?;
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

    fn new(areas: Areas) -> Result<Ui, ErrorString> {
        let sdl_context = sdl2::init()?;
        let video_subsystem = sdl_context.video()?;
        let screen_rect = Ui::get_screen_rect(&video_subsystem)?;
        let window = video_subsystem
            .window(
                &get_binary_name()?,
                screen_rect.width(),
                screen_rect.height(),
            )
            .position(screen_rect.x(), screen_rect.y())
            .borderless()
            .build()?;
        let canvas = window.into_canvas().build()?;
        let event_pump = sdl_context.event_pump()?;
        let mut ui = Ui {
            canvas,
            event_pump,
            ui_elements: areas.ui_elements(screen_rect.width(), screen_rect.height()),
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
        for element in &self.ui_elements {
            let rect = element.0;
            self.canvas.set_draw_color(element.1);
            self.canvas.fill_rect(rect)?;
        }
        self.canvas.present();
        Ok(())
    }
}
