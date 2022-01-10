extern crate x11_clipboard;

use x11_clipboard::Clipboard;
use std::{time::{SystemTime, UNIX_EPOCH, Duration}, thread::JoinHandle};
use serde::{Serialize, Deserialize};
use sys_util::vsock::{VsockSocket, SocketAddr, VsockStream, VsockCid, VsockListener};

use std::io::Write;
use std::io::Read;
use bincode;
use std::env;

const VSOCK_PORT: u32 = 8001;
const VSOCK_CONNECT_DELAY_MS: u64 = 2000;

#[derive(Debug, Serialize, Deserialize)]
pub enum XVsockEventEnum {
    ButtonEvent {
        x: u16,
        y: u16,
        button: u16,
        pressed: bool
    },
    MotionEvent {
        x: u16,
        y: u16
    },
    ConfigureEvent {
        width: u16,
        height: u16
    },
    ClipboardUpdateRequest {
        buf: Vec<u8>
    },
    ClipboardRequestEvent,
    GuestWindowOpened {
        width: u16,
        height: u16
    }
}

pub struct XVsock {
    vsock: Option<VsockStream>,
    clipboard: Clipboard,
    last_vsock_attempt: SystemTime,

    buf_size: Vec<u8>,
    buf_data: Vec<u8>
}

impl XVsock {
    pub fn new() -> XVsock {
        let mut buf_data = vec![0u8; 1000];
        let mut buf_size = vec![0u8; 4];
        XVsock { 
            vsock: None,
            clipboard: Clipboard::new().unwrap(),
            last_vsock_attempt: UNIX_EPOCH,
            buf_size, buf_data
        }
    }

    pub fn connect(&mut self) {
        println!("Openning Vsock for X Event Stream");
        self.vsock = match VsockSocket::new().unwrap().connect(SocketAddr {
            cid: VsockCid::Cid(env::var("VM_CID").unwrap().parse().expect("Failed to parse VM_CID")),
            port: VSOCK_PORT,
        }) {
            Ok(val) => Some(val),
            Err(err) => {
                println!("Failed to open VSock for X Events: {:?}", err);
                None
            },
        };
    }

    pub fn send_clipboard(&mut self, wait: bool) -> () {
        let res = match wait {
            true => self.clipboard.load_wait(
                self.clipboard.getter.atoms.clipboard,
                self.clipboard.getter.atoms.utf8_string,
                self.clipboard.getter.atoms.property
            ),
            false => self.clipboard.load(
                self.clipboard.getter.atoms.clipboard,
                self.clipboard.getter.atoms.utf8_string,
                self.clipboard.getter.atoms.property,
                Duration::from_millis(100)
            )
        };
        match res {
            Ok(buf) => {
                println!("ClipboardUpdateEvent -> buf({})", buf.len());
                self.send(&XVsockEventEnum::ClipboardUpdateRequest {buf});
            }
            Err(err) => {
                self.send(&XVsockEventEnum::ClipboardUpdateRequest {buf: String::from("").into_bytes()});
            }
        }
    }

    pub fn update_clipboard(&self, buf: Vec<u8>) {
        self.clipboard.store(
            self.clipboard.setter.atoms.clipboard, 
            self.clipboard.setter.atoms.utf8_string, 
            buf).expect("Failed to store clipboard");
    }

    pub fn send(&mut self, xvsock_event: &XVsockEventEnum) -> () {
        if self.vsock.is_none() {
            let mut in_ms: u64 = 0;
            let since_last_attempt = SystemTime::now()
                .duration_since(self.last_vsock_attempt)
                .expect("Time went backwards");
            in_ms = since_last_attempt.as_secs() * 1000 +
            since_last_attempt.subsec_nanos() as u64 / 1_000_000;
            if in_ms > VSOCK_CONNECT_DELAY_MS {
                    self.last_vsock_attempt = SystemTime::now();
                    self.connect();
            }
        }
        
        match &mut self.vsock {
            Some(vsock) => {
                let encoded: Vec<u8> = bincode::serialize(&xvsock_event).unwrap();
                let payload_size = encoded.len() as u32;
                let encoded_payload_size: Vec<u8> = bincode::serialize(&payload_size).unwrap();
                match vsock.write_all(&encoded_payload_size)
                    .and_then(|_| vsock.write_all(&encoded)) {
                    Ok(_) => {},
                    Err(ref e) if e.kind() == std::io::ErrorKind::BrokenPipe => {
                        println!("VSock is broken. Invalidating old connection");
                        self.vsock = None;
                    }
                    Err(err) => println!("VSock write failed : {:?}", err),
                }
            }
            None => println!("VSock not opened")
        };
    }

    pub fn read_event(&mut self) -> XVsockEventEnum {
        self.vsock.as_mut().unwrap().read(&mut self.buf_size).expect("Failed to read buf size from socket");
        let decoded_buf_size: u32 = bincode::deserialize(&mut self.buf_size).unwrap();
        
        let buf_packet = &mut self.buf_data[0..(decoded_buf_size as usize)];
        self.vsock.as_mut().unwrap().read(buf_packet).expect("Failed to read buf from socket");
        bincode::deserialize(buf_packet).unwrap()
    }

    pub fn event_loop<Callback: Fn(&mut XVsock, XVsockEventEnum)>(&mut self, callback: &Callback) {

        while self.vsock.is_none() {
            std::thread::sleep(Duration::from_millis(1000));
        }

        loop {
            let xvsock_event = self.read_event();
            callback(self, xvsock_event);
        }
    }

    pub fn listen<Callback: Fn(&mut XVsock, XVsockEventEnum)>(&mut self, callback: Callback) {
        let mut vsock_listener: VsockListener = VsockListener::bind(SocketAddr {
            cid: VsockCid::Any,
            port: VSOCK_PORT,
        }).expect("Failed to listen on vsock port");

        loop {
            let (mut vsock_stream, sock_addr) = vsock_listener.accept().unwrap();
            self.vsock = Some(vsock_stream);
            self.event_loop(&callback);
        }
    }
}