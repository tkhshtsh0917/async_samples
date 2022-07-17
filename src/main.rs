use std::collections::HashMap;
use std::fmt::Debug;
use std::fs::File;
use std::io::{BufWriter, Write};
use std::time::Instant;
use std::thread;
use std::sync::mpsc::{self, Sender, Receiver};

pub enum MessageKind {
    Request(Message),
    Response(Message),
    Terminate,
    HeartBeat(usize),
}

#[derive(Debug, Hash, PartialEq, Eq)]
pub enum HistoryKind {
    Request,
    Response,
}

#[derive(Debug)]
pub struct Message {
    pub name: String,
    pub identifier: usize,
    pub history: HashMap<HistoryKind, String>
}
impl Message {
    fn new(identifier: usize) -> Self {
        Message {
            name: format!("Message #{}", &identifier),
            identifier,
            history: HashMap::new(),
        }
    }
}

const RESULT_FILE_PATH: &str = "result.txt";

fn main() {
    let timer = Instant::now();

    let (ch1_req_tx, ch1_req_rx) = mpsc::channel();
    let (ch2_req_tx, ch2_req_rx) = mpsc::channel();
    let (ch3_req_tx, ch3_req_rx) = mpsc::channel();

    let (ch1_res_tx, ch1_res_rx) = mpsc::channel();
    let (ch2_res_tx, ch2_res_rx) = mpsc::channel();
    let (ch3_res_tx, ch3_res_rx) = mpsc::channel();

    let ch1 = thread::spawn(move || make_task("ch1", ch1_res_tx, ch1_req_rx));
    let ch2 = thread::spawn(move || make_task("ch2", ch2_res_tx, ch2_req_rx));
    let ch3 = thread::spawn(move || make_task("ch3", ch3_res_tx, ch3_req_rx));

    let req_txs = [ch1_req_tx, ch2_req_tx, ch3_req_tx];
    let res_rxs = [ch1_res_rx, ch2_res_rx, ch3_res_rx];
    
    let mut file = BufWriter::new(File::create(RESULT_FILE_PATH).unwrap());

    for step in 0..=600_000 {
        for (i, tx) in req_txs.iter().enumerate() {
            match i == step % 3 {
                true => {
                    let msg = Message::new(step);
                    tx.send(MessageKind::Request(msg)).unwrap()
                },
                false => tx.send(MessageKind::HeartBeat(step)).unwrap(),
            }
        }

        for rx in res_rxs.iter() {
            if let MessageKind::Response(mut msg) = rx.recv().unwrap() {
                msg.history.insert(HistoryKind::Response, String::from("OK"));
                file.write(format!("{:?}\n", msg).as_bytes()).unwrap();
            }
        }
    }

    file.flush().unwrap();

    // Send terminate signals.
    for req_tx in req_txs {
        req_tx.send(MessageKind::Terminate).unwrap();
    }

    ch1.join().unwrap();
    ch2.join().unwrap();
    ch3.join().unwrap();

    println!("{}", timer.elapsed().as_secs_f64());
}

fn make_task(identifier: &str, tx: Sender<MessageKind>, rx: Receiver<MessageKind>) {
    for received in rx {
        match received {
            MessageKind::Request(mut msg) => {
                msg.history.insert(HistoryKind::Request, format!("{}: OK", identifier));

                /*
                if msg.identifier % 10_000 == 0 {
                    println!("{}: Request {:?}", identifier, msg);
                }
                */

                tx.send(MessageKind::Response(msg)).unwrap();
            },
            MessageKind::Response(_) => continue,
            MessageKind::Terminate => break,
            MessageKind::HeartBeat(step) => {
                /*
                if step % 10_000 == 0 {
                    println!("{}: HeartBeat #{}", identifier, step);
                }
                */

                tx.send(MessageKind::HeartBeat(step)).unwrap();
            },
        }
    }
}