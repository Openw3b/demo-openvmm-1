#[path = "../common/config.rs"]
mod config;
#[path = "../common/xvsock.rs"]
mod xvsock;


use std::sync::mpsc::{sync_channel, Receiver, Sender};
use std::time::Duration;
use std::{thread, time, fs};
use std::io::Write;
use std::io::Read;
use std::{fmt, io};
use std::str;
use std::sync::{Mutex, Arc};
use std::sync::mpsc;

use config::AppConfig;
use sys_util::vsock::{VsockSocket, SocketAddr, VsockListener, VsockCid};
use bincode;
use serde::{Serialize, Deserialize};
use x11_clipboard::Clipboard;
use xdotool::command::options::{SearchOption, KeyboardOption};
use xdotool::mouse::Button;
use xdotool::{window, keyboard};
use xdotool::{mouse, option_vec, OptionVec};
use xvsock::{XVsockEventEnum, XVsock};

enum MouseSignals {
    MOVED
}

static mut mouse_pos:Option<Box<Arc<Mutex<(u16,u16)>>>> = None;

struct XVsockServer {
    xvsock: XVsock,
    main_window: Mutex<String>,

    // mouse_pos: Box<Arc<Mutex<(u16,u16)>>>,
    // mouse_chanel: Option<(Sender<MouseSignals>, Arc<Mutex<Receiver<MouseSignals>>>)>
    mouse_signal: Option<Sender<MouseSignals>>
}

impl XVsockServer {
    pub fn new() -> XVsockServer {
        unsafe {
            mouse_pos = Some(Box::new(Arc::new(Mutex::new((0,0)))));
        }
        XVsockServer {
            xvsock: XVsock::new(),
            main_window: Mutex::new(String::from("")),
            // mouse_chanel: None
            mouse_signal: None
        }
    }

    pub async fn get_main_window(&self, config: &Mutex<AppConfig>) {
        thread::sleep(time::Duration::from_millis(6000));
        let output = window::search(config.lock().unwrap().display.main_window.as_str(), option_vec![
            SearchOption::OnlyVisible,
            SearchOption::Name
        ]);
        match output.status.code() {
            Some(0) => {
                let mut main_window = self.main_window.lock().unwrap();
                *main_window = String::from_utf8(output.stdout).unwrap().trim_end_matches('\n').to_string();
                println!("Main Window ID : {:?}", main_window);
            },
            _ => println!("Failed to get focused window (Status:{:?}), Output: {:?}, Error: {:?}", output.status, output.stdout, output.stderr)
        };
    }

    pub fn listen(&mut self) {
        let mouse_signal = self.mouse_signal.as_ref().unwrap();
        self.xvsock.listen(|_xvsock, xvsock_event| {
            match xvsock_event {
                XVsockEventEnum::MotionEvent { x, y } => {
                    unsafe {
                        *mouse_pos.as_mut().unwrap().lock().unwrap() = (x,y);
                        // *new_pos = ;
                    }
                    mouse_signal.send(MouseSignals::MOVED);
                },
                XVsockEventEnum::ButtonEvent { x, y, button, pressed} => {
                    let button_enum = match button {
                        1 => Button::Left,
                        2 => Button::Middle,
                        3 => Button::Right,
                        4 => Button::ScrollUp,
                        5 => Button::ScrollDown,
                        _ => {
                            println!("Unsupported button : {}", button);
                            return;
                        }
                    };
                    mouse::move_mouse(x, y, option_vec![]);
                    if pressed {
                        mouse::click_down(button_enum, option_vec![]);
                    } else {
                        mouse::click_up(button_enum, option_vec![]);
                    }
                },
                XVsockEventEnum::ConfigureEvent { width, height } => {
                    window::set_window_size(self.main_window.lock().unwrap().as_str(), 
                        width.to_string().as_str(), 
                        height.to_string().as_str(), 
                        option_vec![]);
                },
                XVsockEventEnum::ClipboardUpdateRequest { buf } => {
                    println!("Updating Clipboard buffer {:?}", buf);
                    _xvsock.update_clipboard(buf);
                    keyboard::send_key("ctrl+v", option_vec![]);
                },
                XVsockEventEnum::ClipboardRequestEvent {} => {
                    keyboard::send_key("ctrl+c", option_vec![]);
                    _xvsock.send_clipboard(false);
                },
                val => println!("Unsupported event received : {:?}", val)
            }
        });
    }

    pub fn resize_fullscreen(&self) {
        window::set_window_size(self.main_window.lock().unwrap().as_str(), 
        "1920", 
        "1080", 
        option_vec![]);
    }

    pub fn start_mouse_thread(self: &mut XVsockServer) {
        let (tx,rx) = mpsc::channel();
        self.mouse_signal = Some(tx);
        unsafe{
            let _mouse_pos = mouse_pos.as_ref();
            let mut old_pos =(0,0);
            thread::spawn(move || {
                // let rx = rx.lock().unwrap();
                loop {
                    let new_pos = *_mouse_pos.unwrap().lock().unwrap();
                    if new_pos.0 == old_pos.0 && new_pos.1 == old_pos.1 {
                        // Wait for move signal
                        rx.recv().unwrap();
                        // Flush remaining move signals
                        while rx.try_recv().is_ok() {}
                    } else {
                        mouse::move_mouse(new_pos.0, new_pos.1, option_vec![]);
                        old_pos = new_pos;
                    }
                }
            });
        }
    }
}

#[tokio::main]
async fn main() -> io::Result<()>{

    let config_str = fs::read_to_string("/mnt/app.yaml").expect("Unable to read app config file");
    let config: Mutex<AppConfig> = Mutex::new(serde_yaml::from_str(&config_str).unwrap());

    let mut server: XVsockServer = XVsockServer::new();
    unsafe {
        server.get_main_window(&config).await;
        server.resize_fullscreen();
        server.start_mouse_thread();
        server.listen();
    }
    Ok(())
}