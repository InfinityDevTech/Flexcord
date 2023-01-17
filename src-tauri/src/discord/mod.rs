mod consts;

use std::{thread::sleep, time};

use consts::Status;

use serde_json::{json, Value};

use consts::OpCode;
use crossbeam_channel::{unbounded, Receiver};
use futures_util::{stream::SplitSink, SinkExt, StreamExt};
use tokio::net::TcpStream;
use tokio_tungstenite::{
    connect_async,
    tungstenite::{stream::MaybeTlsStream, Message},
    WebSocketStream,
};

/// 
// A lot of this code was shamelessly stolen from (https://github.com/SpaceManiac/discord-rs)
// Go show that repo some love as it works good
///

struct Timer {
    next_tick_at: time::Instant,
    tick_len: time::Duration,
}

impl Timer {
    fn new(tick_len_ms: u64) -> Timer {
        let tick_len = time::Duration::from_millis(tick_len_ms);
        Timer {
            next_tick_at: time::Instant::now() + tick_len,
            tick_len: tick_len,
        }
    }

    #[allow(dead_code)]
    fn immediately(&mut self) {
        self.next_tick_at = time::Instant::now();
    }

    fn defer(&mut self) {
        self.next_tick_at = time::Instant::now() + self.tick_len;
    }

    fn check_tick(&mut self) -> bool {
        if time::Instant::now() >= self.next_tick_at {
            self.next_tick_at = self.next_tick_at + self.tick_len;
            true
        } else {
            false
        }
    }

    fn sleep_until_tick(&mut self) {
        let now = time::Instant::now();
        if self.next_tick_at > now {
            std::thread::sleep(self.next_tick_at - now);
        }
        self.next_tick_at = self.next_tick_at + self.tick_len;
    }
}

pub struct ConnectionBuilder {
    url: String,
    token: String,
}

impl ConnectionBuilder {
    pub fn new(url: String, token: String) -> ConnectionBuilder {
        ConnectionBuilder { url, token }
    }

    pub fn connect(&self) -> Result<(Connection, ReadyEvent)> {
        let info = os_info::get();
        let mut packet = json!({
           "op": OpCode::Identify,
           "d": {
            "token": self.token,
            "properties": {
                "$os": info.os_type(),
                "$browser": "Flexcord client",
                "$device": "Flexcord client",
            },
            "large_threshold": 250,
            "v": 10
           }
        });
        Connection::_connect(&self.url, self.token.clone(), packet);
    }
}

pub struct Connection {}

impl Connection {
    pub async fn _connect(
        url: &str,
        token: String,
        packet: Value,
    ) -> Result<(Connection, ReadyEvent)> {
        let (ws, _) = connect_async(url).await.expect("Failure to connect = bad.");
        let (mut send, mut receiver) = ws.split();

        send.send(&packet);

        let helloE;
        match json::parse(receiver.next()) {
            Ok(v) => helloE = v,
            other => {
                println!("Unexpected data in connection! - {:?}", other);
                panic!("Unexpected data in connection! -{:?}", other);
            }
        }

        let (tx, rx) = unbounded();
        std::thread::Builder::new()
            .name("Discord sender".into())
            .spawn(move || keepalive(helloE.clone(), send, rx));

        let seq;
        let ready;
    }
}

fn keepalive(
    helloE: Value,
    mut sender: SplitSink<WebSocketStream<MaybeTlsStream<TcpStream>>, Message>,
    rx: Receiver<Value>,
) {
    let interval = helloE["d"]["heartbeat_interval"];
    let mut timer = Timer::new(interval);
    let mut last_s = 0;

    'mLoop: loop {
        sleep(time::Duration::from_millis(10));

        loop {
            match rx.try_recv() {
                Ok(Status::SendMessage(val)) => match sender.send(&val) {
                      Ok(()) => {},
                      Err(err) => println!("Error sending message {}", err),
                },
                Ok(Status::Sequence(seq)) => {
                    last_s = seq;
                },
                Ok(Status::ChangeInterval(interval)) => {
                    timer = Timer::new(interval);
                },
                Ok(Status::ChangeSender(new_sender)) => {
                    sender = new_sender;
                },
                Ok(Status::Aborted) => break 'mLoop,
                Err(crossbeam_channel::TryRecvError::Empty) => break,
                Err(crossbeam_channel::TryRecvError::Disconnected) => break 'mLoop,
            }
        }

        if timer.check_tick() {
            let map = json!({
                "op": 1,
                "d": last_s,
            });
            match sender.send(&map) {
                Ok(()) => {},
                Err(err) => println!("Error sending heartbeat {}", err)
            }
        }
    }
    let _ = sender.get_mut().shutdown(std::net::Shutdown::Both);
}
