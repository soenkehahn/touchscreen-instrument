extern crate sdl2;

use self::sdl2::EventPump;
use self::sdl2::event::Event;
use self::sdl2::keyboard::Keycode;
use self::sdl2::pixels::Color;
use self::sdl2::rect::Rect;
use self::sdl2::render::Canvas;
use self::sdl2::video::Window;
use ErrorString;
use areas::Areas;
use areas::Rectangle;
use get_binary_name;

pub const SCREEN_WIDTH: u32 = 1920;
pub const SCREEN_HEIGHT: u32 = 1080;

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
    ui_elements: Vec<(Rectangle, ::areas::Color)>,
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

    fn new(areas: Areas) -> Result<Ui, ErrorString> {
        let sdl_context = sdl2::init()?;
        let video_subsystem = sdl_context.video()?;
        let window = video_subsystem
            .window(&get_binary_name()?, SCREEN_WIDTH, SCREEN_HEIGHT)
            .borderless()
            .build()?;
        let canvas = window.into_canvas().build()?;
        let event_pump = sdl_context.event_pump()?;
        let mut ui = Ui {
            canvas,
            event_pump,
            ui_elements: areas.ui_elements(),
        };
        ui.move_to_touch_screen();
        ui.draw()?;
        Ok(ui)
    }

    fn move_to_touch_screen(&self) {
        // sdl doesn't support controlling which screen to put a window on.
        match ::std::process::Command::new("xdotool")
            .args(&["getactivewindow", "windowmove", "1366", "0"])
            .output()
        {
            Err(e) => {
                eprintln!("error executing xdotool: {:?}", e);
            }
            Ok(output) => {
                if !output.status.success() {
                    eprintln!("xdotool error: {:?}", output);
                }
            }
        }
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

    fn convert_color(color: &::areas::Color) -> Color {
        Color::RGB(color.red, color.green, color.blue)
    }

    fn draw(&mut self) -> Result<(), ErrorString> {
        self.canvas.set_draw_color(Color::RGB(0, 0, 0));
        self.canvas.clear();
        for element in &self.ui_elements {
            match element.0 {
                Rectangle { x, y, w, h } => {
                    self.canvas.set_draw_color(Ui::convert_color(&element.1));
                    self.canvas.fill_rect(Rect::new(x, y, w as u32, h as u32))?;
                }
            }
        }
        self.canvas.present();
        Ok(())
    }
}
