// 8.2.1

use std::cell::RefCell;
use std::sync::Arc;
use std::thread;
use rustc_serialize::json;
use std::str;
use mio::*;
use mio::udp::*;
use std::net::ToSocketAddrs;
use mio::buf::{RingBuf, SliceBuf, MutSliceBuf};

type SeqNum = u64;

#[derive(PartialEq, Clone)]
#[derive(RustcEncodable, RustcDecodable)]
struct Guid(pub u64);

#[derive(RustcEncodable, RustcDecodable)]
struct Locator(pub u64);

trait Entity {
    fn get_guid() -> Guid;
}

enum ProtocolId {
    Rtps,
}

struct Header {
    protocol:u8,
    version:u8,
    vendor_id:u8,
    guid_prefix:u8,
}

#[derive(RustcEncodable, RustcDecodable)]
#[derive(Debug)]
enum SubmessageKind {
    Data,
    DataFrag,
    Gap,
    Heartbeat,
    AckNack,
    HeartbeatFrag,
    NackFrag,
    Pad,
    InfoTimestamp,
    InfoReply,
    InfoDestination,
    InfoSource,
}

struct SubmessageHeader {
    submessage_id:SubmessageKind,
    flags:u8,
    length:u8,
}

struct SubmessageElement;

struct Submessage {
    header:SubmessageHeader,
    element:SubmessageElement,
}

struct EndPoint {
    topic_kind:u8,
    reliability_level:u8,
    unicast_locator_list:Vec<Locator>,
    multicast_locator_list:Vec<Locator>,
}

struct Participant {
    protocol_version:u8,
    vendor_id:u8,
    default_unicast_locator_list:Vec<Locator>,
    default_multicast_locator_list:Vec<Locator>,
}

struct Writer {
    guid:Guid,
    sequence_number:SeqNum,
    history_cache:HistoryCache,

    _target:Option<Arc<RefCell<Reader>>>,
}

impl Writer {
    fn new() -> Writer {
        Writer {
            guid: Guid(44),
            sequence_number: 0,
            history_cache: HistoryCache::new(),
            _target: None,
        }
    }

    fn new_change(&mut self) -> CacheChange {
        let seq = self.sequence_number;
        self.sequence_number += 1;

        CacheChange {
            kind: ChangeKind::Kind,
            writer_guid: self.guid.clone(),
            sequence_number: seq,
            data: vec![],
        }
    }
}

struct Reader {
    history_cache:HistoryCache,
}

impl Reader {
    fn new() -> Reader {
        Reader {
            history_cache: HistoryCache::new(),
        }
    }

    fn _message(&mut self, message:SubmessageKind) {
        match message {
            SubmessageKind::Data => {
                self.history_cache.add_change(CacheChange {
                    kind: ChangeKind::Kind,
                    writer_guid: Guid(0),
                    sequence_number: 0,
                    data: vec![],
                });

                // ...The DDS user is notified by one of the mechanisms described in the DDS Specification

                // RESPOND with AckNack { reader_guid, seq_num_change, }
            },
            _ => { }
        }
    }
}

#[derive(PartialEq)]
#[derive(RustcEncodable, RustcDecodable)]
enum ChangeKind {
    Kind,
}

#[derive(PartialEq)]
#[derive(RustcEncodable, RustcDecodable)]
struct CacheChange {
    kind:ChangeKind,
    writer_guid:Guid,
    //instance_handle
    sequence_number:SeqNum,
    data:Vec<u8>,
}

type Status = Result<(), ()>;

#[allow(non_upper_case_globals)]
const Success:Status = Ok(());
#[allow(non_upper_case_globals)]
const Failure:Status = Err(());

struct HistoryCache {
    changes:Vec<CacheChange>,
}

impl HistoryCache {
    fn new() -> HistoryCache {
        HistoryCache {
            changes: vec![]
        }
    }

    fn add_change(&mut self, change:CacheChange) -> Status {
        self.changes.push(change);
        Success
    }

    fn remove_change(&mut self, change:CacheChange) -> Status {
        if let Some(pos) = self.changes.iter().position(|r| *r == change) {
            self.changes.remove(pos);
        }
        Success
    }

    // fn get_change(&self) {}

    fn get_seq_num_min(&self) -> Option<SeqNum> {
        self.changes.iter().map(|r| r.sequence_number).min()
    }

    fn get_seq_num_max(&self) -> Option<SeqNum> {
        self.changes.iter().map(|r| r.sequence_number).max()
    }
}

fn send_socket(tx:&UdpSocket, msg:&SubmessageKind) {
    let mut buf = RingBuf::new(1024);
    buf.write_slice(json::encode(msg).unwrap().as_bytes());
    tx.send_to(&mut buf, &"127.0.0.1:7556".to_socket_addrs().unwrap().next().unwrap()).unwrap();
}

fn recv_socket(rx:&UdpSocket) -> SubmessageKind {
    let mut buf = RingBuf::new(1024);
    rx.recv_from(&mut buf).unwrap();
    json::decode(str::from_utf8(buf.bytes()).unwrap()).unwrap()
}

struct TxHandler {
    tx: UdpSocket,
    writer: Writer,
}

impl TxHandler {
    fn new(writer:Writer, tx: UdpSocket) -> TxHandler {
        TxHandler {
            tx: tx,
            writer: writer,
        }
    }
}

impl Handler for TxHandler {
    type Timeout = usize;
    type Message = ();
    
    fn ready(&mut self, event_loop: &mut EventLoop<TxHandler>, _: Token, events: EventSet) {
        if events.is_writable() {
            debug!("We are writing a datagram now...");
            send_socket(&self.tx, &SubmessageKind::Data);
            send_socket(&self.tx, &SubmessageKind::Heartbeat);
            event_loop.shutdown();
        }
    }
}

struct RxHandler {
    rx: UdpSocket,
    reader: Reader,
}

impl RxHandler {
    fn new(reader:Reader, rx: UdpSocket) -> RxHandler {
        RxHandler {
            rx: rx,
            reader: reader,
        }
    }
}

impl Handler for RxHandler {
    type Timeout = usize;
    type Message = ();
    
    fn ready(&mut self, event_loop: &mut EventLoop<RxHandler>, _: Token, events: EventSet) {
        if events.is_readable() {
            debug!("We are receiving a datagram now...");
            self.reader._message(recv_socket(&self.rx));
            event_loop.shutdown();
        }
    }
}

#[test]
fn test_8_4_1_1() {
    let a = thread::spawn(move || {
        let mut event_loop = EventLoop::new().unwrap();

        let mut writer = Writer::new();

        let change = writer.new_change();
        writer.history_cache.add_change(change);

        // on writer's thread...
        // TODO: history cache thread or writer thread?

        let tx = UdpSocket::bound(&"127.0.0.1:7555".to_socket_addrs().unwrap().next().unwrap()).unwrap();
        // let tx:UdpSocket = UdpSocket::bind("127.0.0.1:7555").unwrap();

        event_loop.register_opt(&tx, Token(1), EventSet::writable(), PollOpt::edge()).unwrap();
        // send_socket(&tx, &SubmessageKind::Data);
        // send_socket(&tx, &SubmessageKind::Heartbeat);
        
        // drop(tx); // close the socket
        event_loop.run(&mut TxHandler::new(writer, tx)).unwrap();
    });

    let b = thread::spawn(move || {
        let mut event_loop = EventLoop::new().unwrap();

        let reader = Reader::new();
        let rx = UdpSocket::bound(&"127.0.0.1:7556".to_socket_addrs().unwrap().next().unwrap()).unwrap();
        // let rx:UdpSocket = UdpSocket::bind("127.0.0.1:7556").unwrap();

        event_loop.register_opt(&rx, Token(0), EventSet::readable(), PollOpt::edge()).unwrap();

        event_loop.run(&mut RxHandler::new(reader, rx)).unwrap();

        // reader._message(recv_socket(&rx));
        // reader._message(recv_socket(&rx));

        // drop(rx); // close the socket

        // The StatefulWriter records that the RTPS Reader has received the CacheChange and adds it to the set of
        // acked_changes maintained by the ReaderProxy using the acked_changes_set operation
    });

    let _ = a.join();
    let _ = b.join();
}
