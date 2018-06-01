extern crate sdl;

use self::sdl::event::{Event, Key};
use self::sdl::video::{Surface, SurfaceFlag, VideoFlag};
use areas::Areas;

const SCREEN_WIDTH: u16 = 1920;
const SCREEN_HEIGHT: u16 = 1080;

impl Areas {
    pub fn spawn_ui(self) {
        ::std::thread::spawn(move || {
            Ui::run_ui();
        });
    }
}

struct Ui {
    surface: Surface,
}

impl Ui {
    fn run_ui() {
        let ui = Ui::new();
        ui.run_main_loop();
        ui.quit();
    }

    fn new() -> Ui {
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
        let ui = Ui { surface };
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

    fn draw(&self) {
        let rect_size: u16 = 100;
        let blue = sdl::video::Color::RGB(0, 0, 255);
        self.surface.fill_rect(
            Some(sdl::Rect {
                x: 0,
                y: 0,
                w: rect_size,
                h: rect_size,
            }),
            blue,
        );
        self.surface.fill_rect(
            Some(sdl::Rect {
                x: (SCREEN_WIDTH - rect_size) as i16,
                y: (SCREEN_HEIGHT - rect_size) as i16,
                w: SCREEN_WIDTH,
                h: SCREEN_HEIGHT,
            }),
            blue,
        );
        self.surface.flip();
    }
}
