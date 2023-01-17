extern crate chrono;
extern crate json;

use crossbeam_channel::{unbounded, Receiver, Sender};
use futures_util::stream::SplitSink;
use futures_util::{SinkExt, StreamExt};
use json::{parse, JsonValue, JsonError};
use reqwest::header::{HeaderMap, HeaderValue};
use std::thread::{self, sleep};
use std::time::{Duration, SystemTime, UNIX_EPOCH};
use tokio::net::TcpStream;
use tokio_tungstenite::tungstenite::Message;
use reqwest::{get, Client, Url};
use tokio_tungstenite::{connect_async, MaybeTlsStream, WebSocketStream};

// This is called in a new thread.
pub async fn start_client() {
    let (socket, _) = connect_async("wss://gateway.discord.gg/?v=10&encoding=json")
        .await
        .expect("Unable to connect!");
    let (mut write, read) = socket.split();
    let identfy = json::parse(r#"{"op":2,"d":{"token":"---","properties":{"$os":"windows","$browser":"Discord","$device":"Flexcord"},"large_threshold":{"isLosslessNumber":true,"value":"250"},"compress":false,"v":10}}"#).expect("Unable to parse identify payload!");
    write
        .send(Message::Text(json::stringify(identfy).into()))
        .await
        .unwrap();
    let (c, x): (Sender<JsonValue>, Receiver<JsonValue>) = unbounded();
    let (send_message, to_send): (Sender<JsonValue>, Receiver<JsonValue>) = unbounded();
    thread::Builder::new()
        .name("Socket Reader".to_string())
        .spawn(|| {
            let c = c;
            let rt = tokio::runtime::Runtime::new().unwrap();
            let d = read.for_each(|message| async {
                let data: Vec<u8> = message.unwrap().into_data();
                let _ = &c.send(parse(std::str::from_utf8(&data).unwrap()).unwrap())
                    .unwrap();
                let msg = parse(std::str::from_utf8(&data).unwrap()).unwrap();
                if msg["t"] == "READY" {
                    println!("Logged in as {}", msg["d"]["user"]["username"]);
                    println!("Current email is {}", msg["d"]["user"]["email"]);
                    println!("Flags {}", msg["d"]["user"]["flags"]);
                }
            });
            rt.block_on(d);
        }).expect("Unable to create heartbeat thred. Client would close anyways due to lack of heartbeat.");
    let msg = x.recv().expect("Unwrap Err");
    if msg["op"] == 10 {
        let _ = write.send(Message::Text("{\"op\": 1, \"d\": null}".to_string()));
        thread::Builder::new()
        .name("Socket Heartbeat".to_string())
        .spawn(move || {
            let rt = tokio::runtime::Runtime::new().unwrap();
            rt.block_on(
                heartbeat(msg["d"]["heartbeat_interval"].as_u64().unwrap(), write, to_send));
        }).expect("Unable to create heartbeat thred. Client would close anyways due to lack of heartbeat.");
    } else {
        panic!("Invalid OP code");
    }
    loop {
        //println!("{}", x.recv().unwrap());
        let mut message = x.recv().unwrap();
        if message["t"] == "MESSAGE_CREATE" {
            let guild = getGuild(message["d"]["guild_id"].as_str().unwrap()).await.unwrap();
            println!("{} - {}#{}: {}", guild["name"], message["d"]["author"]["username"], message["d"]["author"]["discriminator"], message["d"]["content"]);
        }
        message["d"] = json::parse("{}").unwrap();
        println!("Sending {}", message);
        send_message.send(message).expect("My bad");
    }
}

pub async fn heartbeat(
    interval: u64,
    mut writer: SplitSink<WebSocketStream<MaybeTlsStream<TcpStream>>, Message>,
    tosend: Receiver<JsonValue>,
) {
    let mut lasttime = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap()
        .as_millis();
    let mut last_sequence: i64 = 0;

    'outer: loop {
        sleep(Duration::from_millis(1));

        loop {
            match tosend.try_recv() {
                Ok(msg) => {
                    if msg["op"] == 1 {
                        println!(
                            "------- Setting sequence to  {}",
                            msg["s"].as_i64().unwrap()
                        );
                        last_sequence = msg["s"].as_i64().unwrap();
                        writer
                            .send(Message::Text(format!(
                                "{{\"op\": 1, \"d\": {}}}",
                                last_sequence
                            )))
                            .await
                            .unwrap();
                    }
                    if msg["s"].as_i64() >= Some(0) {
                        println!(
                            "------- Setting sequence to  {}",
                            msg["s"].as_i64().unwrap()
                        );
                        last_sequence = msg["s"].as_i64().unwrap();
                    }
                }
                Err(_) => break,
            }
        }

        if SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_millis()
            - lasttime
            >= interval as u128
        {
            println!(
                "-------------------- SENDING HEARTBEAT -------------------- Sequence number: {}",
                last_sequence
            );
            writer
                .send(Message::Text(format!(
                    "{{\"op\": 1, \"d\": {}}}",
                    last_sequence
                )))
                .await
                .unwrap();
            lasttime = SystemTime::now()
                .duration_since(UNIX_EPOCH)
                .unwrap()
                .as_millis();
        }
    }
}


pub async fn getGuild(id: &str) -> Result<JsonValue, JsonError> {
    let url = format!("https://discord.com/api/v8/guilds/{}", id);
    let client = reqwest::Client::new();
    let res = client
        .get(&url)
        .header("Authorization", "---");
    let resp = res.send().await.unwrap();
    let guild = json::parse(&resp.text().await.unwrap());
    return guild
}

#[tauri::command]
pub async fn getGuildById(id: String) -> String {
println!("Done!");
   let data = getGuild(&id).await.unwrap();
   return json::stringify(data);
}