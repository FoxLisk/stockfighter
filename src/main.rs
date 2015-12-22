#![feature(associated_consts)]

extern crate hyper;
extern crate rustc_serialize;

use rustc_serialize::{json, Decodable, Decoder};
use hyper::Client;
use hyper::client::response::Response;
use hyper::header;
use std::io::{Read, Error};
use std::fs::File;
use std::time::Duration;
use std::process;
use std::fmt::Debug;

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

#[derive(RustcDecodable, Eq, PartialEq, Debug)]
struct Bid {
    price: u32,
    qty: u32,
}

#[derive(RustcDecodable, Eq, PartialEq, Debug)]
struct Ask {
    price: u32,
    qty: u32,
}

//impl Decodable for Ask {
    //fn decode<D: Decoder>(d: &mut D) -> Result<Self, D::Error> {
        //match d.read_struct
//
    //}
//}
//
//impl Decodable for Bid {
    //fn decode<D: Decoder>(d: &mut D) -> Result<Self, D::Error> {
//
    //}
//}

#[derive(RustcDecodable, Eq, PartialEq, Debug)]
struct OrderBook {
    ok: bool,
    venue: String,
    symbol: String,
    bids: Option<Vec<Bid>>,
    asks: Option<Vec<Ask>>,
    ts: String,
}


#[derive(RustcDecodable, Eq, PartialEq, Debug)]
struct StockSymbol {
    name: String,
    symbol: String
}

#[derive(RustcDecodable, Eq, PartialEq, Debug)]
struct StocksOnVenueResp {
    ok: bool,
    symbols: Vec<StockSymbol>
}

#[derive(RustcDecodable, Eq, PartialEq, Debug)]
struct VenueUpResp {
    ok: bool,
    venue: String
}

#[derive(RustcDecodable, Eq, PartialEq, Debug)]
struct HealthCheckResp {
    ok: bool,
    error: String
}

fn resp_to_obj<T: Decodable + Debug>(maybe_json: &mut Response) -> Option<T> {
    let mut buf = String::new();
    match maybe_json.read_to_string(&mut buf) {
        Ok(_) => {
            match json::decode::<T>(&buf) {
                Ok(blob) => Some(blob),
                Err(e) => { debug!(e) ; debug!(buf) ; None }
            }
        }
        Err(_) => None
    }

}

#[derive(RustcDecodable, Eq, PartialEq, Debug)]
struct LevelResp {
    // XXX there are more fields to care about here later
    ok: bool,
    instanceId: u32,
    account: String,
    venues: Vec<String>,
}

struct StockFighterClient {
    client: Client,
    apikey: String
}

impl StockFighterClient {
    const URL: &'static str = "https://api.stockfighter.io/ob/api";
    const LEVEL_URL: &'static str = "https://stockfighter.io/gm";

    fn new(apikey: String) -> Self {
        let mut client = Client::new();
        client.set_read_timeout(Some(Duration::from_millis(1500)));
        StockFighterClient {
            client: client, apikey: apikey
        }
    }

    fn start_level(&self, level: &str) -> Option<LevelResp> {
        let url = format!("{}/levels/{}", Self::LEVEL_URL, level);
        let mut headers = header::Headers::new();
        let cookiepair = header::CookiePair::new("apikey".to_owned(), (&self.apikey).clone());
        headers.set(header::Cookie(vec![cookiepair]));
        debug!(headers);
        let req = self.client.post(&url).headers(headers);
        let resp = req.send();
        resp_to_obj::<LevelResp>(&mut resp.unwrap())
    }

    fn health_check(&self) -> bool {
        let url = format!("{}/heartbeat", Self::URL);
        let resp = self.client.get(&url).send();
        match resp_to_obj::<HealthCheckResp>(&mut resp.unwrap()) {
            Some(obj) => {
                obj.ok
            },
            None => false
        }
    }

    fn venue_health_check(&self, venue: &str) -> bool {
        let url = format!("{}/venues/{}/heartbeat", Self::URL, venue);
        let resp = self.client.get(&url).send();
        match resp_to_obj::<VenueUpResp>(&mut resp.unwrap()) {
            Some(obj) => obj.ok,
            None => false
        }
    }

    fn stocks_on_venue(&self, venue: &str) -> Option<Vec<StockSymbol>> {
        let url = format!("{}/venues/{}/stocks", Self::URL, venue);
        let resp = self.client.get(&url).send();
        let body = match resp_to_obj::<StocksOnVenueResp>(&mut resp.unwrap()) {
            Some(obj) => obj,
            None => {
                return None
            }
        };
        if !body.ok {
            return None
        }
        return Some(body.symbols);
    }

    fn orderbook_for(&self, venue: &str, stock: &StockSymbol) -> Option<OrderBook> {
        let url = format!("{}/venues/{}/stocks/{}", Self::URL, venue, stock.symbol);
        debug!(url);
        let resp = self.client.get(&url).send();
        match resp_to_obj::<OrderBook>(&mut resp.unwrap()) {
            Some(obj) => Some(obj),
            None => None,
        }

    }
}

fn get_apikey() -> Result<String, Error> {
    let mut f = try!(File::open("cfg/apikey"));
    let mut buf = String::new();
    match f.read_to_string(&mut buf) {
        Ok(_) => {
            Ok(buf.trim().to_owned())
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
    let lvl = match client.start_level(&"chock_a_block") {
        Some(l) => l,
        None => { println!("Level failed to start!"); process::exit(1) }
    };
    debug!(lvl);
    if !client.venue_health_check(&lvl.venues[0]) {
        println!("Unable to access {}", lvl.venues[0]);
        process::exit(1);
    }
    let stocks = match client.stocks_on_venue(&lvl.venues[0]) {
        Some(s) => s,
        None => { println!("Stock return bad?"); process::exit(1); }
    };
    debug!(stocks);
    let book = client.orderbook_for(&lvl.venues[0], &stocks[0]);
    debug!(book);
}

#[cfg(test)]
mod tests {
    use super::StockSymbol;
    use rustc_serialize::json;

    #[test]
    fn test_decode_symbol() {
        println!("hi");
        let decoded = json::decode::<StockSymbol>(&"{\"name\": \"foo\", \"symbol\": \"FOOSYM\"}".to_owned());
        match decoded {
            Ok(obj) => assert_eq![
                StockSymbol { name: "foo".to_owned(), symbol: "FOOSYM".to_owned() },
                obj],
            Err(_) => panic!()
        }

    }

    #[test]
    fn test_decode_symbol_but_with_fancy_string_literal() {
        println!("hi");
        let decoded = json::decode::<StockSymbol>(&r#"{"name": "foo", "symbol": "FOOSYM"}"#.to_owned());
        match decoded {
            Ok(obj) => assert_eq![
                StockSymbol { name: "foo".to_owned(), symbol: "FOOSYM".to_owned() },
                obj],
            Err(_) => panic!()
        }

    }
}

