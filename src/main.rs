#![feature(associated_consts)]

extern crate hyper;
extern crate rustc_serialize;

use rustc_serialize::json;
use hyper::Client;
use hyper::client::response::Response;
use std::io::{Read, Error};
use std::fs::File;
use std::time::Duration;
use std::process;

macro_rules! debug {
    ($s:expr) => (println!("{:?}", $s))
}


// this is sort of a monad implementation lol
macro_rules! pipe {
    ($x:ident | $( $y:ident ($($arg:expr),*) )|+) =>
     {
         $x $(. $y( $($arg),* ).unwrap())+
     }
}

struct StockFighterClient {
    client: Client,
    apikey: String
}

fn resp_to_json(maybe_json: &mut Response) -> Option<json::Json> {
    let mut buf = String::new();
    match maybe_json.read_to_string(&mut buf) {
        Ok(_) => {
            match json::Json::from_str(&buf) {
                Ok(blob) => Some(blob),
                Err(_) => None
            }
        }
        Err(_) => None
    }
}

impl StockFighterClient {
    const URL: &'static str = "https://api.stockfighter.io/ob/api";

    fn new(apikey: String) -> Self {
        let mut client = Client::new();
        client.set_read_timeout(Some(Duration::from_millis(1500)));
        StockFighterClient {
            client: client, apikey: apikey
        }
    }

    fn health_check(&self) -> bool {
        let url = format!("{}/heartbeat", Self::URL);
        let resp = self.client.get(&url).send();
        match resp_to_json(&mut resp.unwrap()) {
            Some(obj) => {
                pipe!(obj | as_object() | get("ok") | as_boolean())
            },
            None => false
        }
    }

    fn venue_health_check(&self, venue: &str) -> bool {
        let url = format!("{}/venues/{}/heartbeat", Self::URL, venue);
        let resp = self.client.get(&url).send();
        match resp_to_json(&mut resp.unwrap()) {
            Some(obj) => pipe!(obj | as_object() | get("ok") | as_boolean()),
            None => false
        }
    }

}

fn get_apikey() -> Result<String, Error> {
    let mut f = try!(File::open("cfg/apikey"));
    let mut buf = String::new();
    match f.read_to_string(&mut buf) {
        Ok(_) => {
            Ok(buf)
        },
        Err(e) => {
            Err(e)
        }
    }
}

fn main() {
    let apikey = match get_apikey() {
        Ok(key) => key,
        Err(msg) => { println!("{}", msg); process::exit(2) }
    };
    let client = StockFighterClient::new(apikey);
    if !client.health_check() {
        println!("Healthcheck down.");
        process::exit(1);
    }
    if !client.venue_health_check(&"TESTEX") {
        println!("TESTEX down.");
        process::exit(1);
    }
}
