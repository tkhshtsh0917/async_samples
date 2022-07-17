use std::collections::HashMap;
use std::fmt::Debug;
use std::fs::File;
use std::io::{BufWriter, Write};
use std::thread;
use std::sync::mpsc::{self, Sender, Receiver};

pub enum MessageKind {
    Request(Message),
    Response(Message),
    Terminate,
    Notify(String),
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
    let (ch1_req_tx, ch1_req_rx) = mpsc::channel();
    let (ch2_req_tx, ch2_req_rx) = mpsc::channel();
    let (ch3_req_tx, ch3_req_rx) = mpsc::channel();

    let (ch1_res_tx, all_res_rx) = mpsc::channel();
    let ch2_res_tx = ch1_res_tx.clone();
    let ch3_res_tx = ch1_res_tx.clone();

    let ch1 = thread::spawn(move || make_task("ch1", ch1_res_tx, ch1_req_rx));
    let ch2 = thread::spawn(move || make_task("ch2", ch2_res_tx, ch2_req_rx));
    let ch3 = thread::spawn(move || make_task("ch3", ch3_res_tx, ch3_req_rx));

    let mut file = BufWriter::new(File::create(RESULT_FILE_PATH).unwrap());
    let req_txs = [ch1_req_tx, ch2_req_tx, ch3_req_tx];

    for step in 0..=600_000 {
        let index = step % 3;
        let msg = Message::new(step);

        req_txs[index].send(MessageKind::Request(msg)).unwrap();

        if step > 0 && step % 1_000 == 0 {
            let info = format!("*** Step = {} ***", step);
            req_txs[index].send(MessageKind::Notify(info)).unwrap();
        }

        for received in all_res_rx.iter() {
            match received {
                MessageKind::Response(mut msg) => {
                    msg.history.insert(HistoryKind::Response, String::from("OK"));
                    file.write(format!("{:?}\n", msg).as_bytes()).unwrap();
                },
                MessageKind::Notify(msg) => println!("{}", msg),
                _ => continue,
            }
            break;
        }
    }

    // Send terminate signals.
    for req_tx in req_txs {
        req_tx.send(MessageKind::Terminate).unwrap();
    }
    
    file.flush().unwrap();

    ch1.join().unwrap();
    ch2.join().unwrap();
    ch3.join().unwrap();
}

fn make_task(identifier: &str, tx: Sender<MessageKind>, rx: Receiver<MessageKind>) {
    for received in rx {
        match received {
            MessageKind::Request(mut msg) => {
                msg.history.insert(HistoryKind::Request, format!("{}: OK", identifier));
                tx.send(MessageKind::Response(msg)).unwrap();
            },
            MessageKind::Response(_) => continue,
            MessageKind::Terminate => {
                tx.send(MessageKind::Notify(format!("{}: Terminated", identifier))).unwrap();
                break;
            },
            MessageKind::Notify(msg) => println!("{}: {}", identifier, msg),
        }
    }

    tx.send(MessageKind::Notify(format!("{}: Done!", identifier))).unwrap();
}