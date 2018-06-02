extern crate sdl;

use self::sdl::event::{Event, Key};
use self::sdl::video::{Surface, SurfaceFlag, VideoFlag};
use areas::Areas;
use areas::Color;
use areas::Rectangle;

pub const SCREEN_WIDTH: u16 = 1920;
pub const SCREEN_HEIGHT: u16 = 1080;

impl Areas {
    pub fn spawn_ui(self) {
        ::std::thread::spawn(move || {
            Ui::run_ui(self);
        });
    }
}

struct Ui {
    surface: Surface,
    ui_elements: Vec<(Rectangle, Color)>,
}

impl Ui {
    fn run_ui(areas: Areas) {
        let ui = Ui::new(areas);
        ui.run_main_loop();
        ui.quit();
    }

    fn new(areas: Areas) -> Ui {
        sdl::init(&[sdl::InitFlag::Video]);
        let surface = match sdl::video::set_video_mode(
            SCREEN_WIDTH as isize,
            SCREEN_HEIGHT as isize,
            32,
            &[SurfaceFlag::HWSurface],
            &[
                VideoFlag::DoubleBuf,
                VideoFlag::Resizable,
                VideoFlag::NoFrame,
            ],
        ) {
            Ok(surface) => surface,
            Err(err) => panic!("failed to set video mode: {}", err),
        };
        let ui = Ui {
            surface,
            ui_elements: areas.ui_elements(),
        };
        ui.move_to_touch_screen();
        ui.draw();
        ui
    }

    fn move_to_touch_screen(&self) {
        // sdl doesn't support controlling which screen to put a window on.
        match ::std::process::Command::new("xdotool")
            .args(&["getactivewindow", "windowmove", "0", "0"])
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

    fn run_main_loop(&self) {
        'main: loop {
            'event: loop {
                let event = sdl::event::wait_event();
                match event {
                    Event::Quit => break 'main,
                    Event::None => break 'event,
                    Event::Key(Key::Escape, _, _, _) => break 'main,
                    Event::Resize(_, _) => {
                        self.draw();
                    }
                    _ => {}
                }
            }
        }
    }

    fn quit(&self) {
        sdl::quit();
        ::std::process::exit(0);
    }

    fn convert_color(&self, color: &Color) -> sdl::video::Color {
        sdl::video::Color::RGB(color.red, color.green, color.blue)
    }

    fn draw(&self) {
        for element in &self.ui_elements {
            match element.0 {
                Rectangle { x, y, w, h } => {
                    self.surface.fill_rect(
                        Some(sdl::Rect {
                            x: x as i16,
                            y: y as i16,
                            w: w as u16,
                            h: h as u16,
                        }),
                        self.convert_color(&element.1),
                    );
                }
            }
        }
        self.surface.flip();
    }
}
