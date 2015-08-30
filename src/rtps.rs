// 8.2.1

use std::cell::RefCell;
use std::sync::Arc;
use std::thread;
use std::net::UdpSocket;
use rustc_serialize::json;
use std::str;

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
    tx.send_to(json::encode(msg).unwrap().as_bytes(), &"127.0.0.1:7556");
}

fn recv_socket(rx:&UdpSocket) -> SubmessageKind {
    let mut buf = [0; 256];
    let (amt, _) = rx.recv_from(&mut buf).unwrap();
    json::decode(str::from_utf8(&buf[0..amt]).unwrap()).unwrap()
}

#[test]
fn test_8_4_1_1() {
    let a = thread::spawn(move || {
        let mut writer = Writer::new();

        // writer._target = Some(Arc::new(RefCell::new(reader)));

        let change = writer.new_change();
        writer.history_cache.add_change(change);

        // on writer's thread...
        // TODO: history cache thread or writer thread?

        let tx:UdpSocket = UdpSocket::bind("127.0.0.1:7555").unwrap();

        send_socket(&tx, &SubmessageKind::Data);
        send_socket(&tx, &SubmessageKind::Heartbeat);
        
        drop(tx); // close the socket
    });

    let b = thread::spawn(move || {
        let mut reader = Reader::new();
        let rx:UdpSocket = UdpSocket::bind("127.0.0.1:7556").unwrap();

        reader._message(recv_socket(&rx));
        reader._message(recv_socket(&rx));

        drop(rx); // close the socket

        // The StatefulWriter records that the RTPS Reader has received the CacheChange and adds it to the set of
        // acked_changes maintained by the ReaderProxy using the acked_changes_set operation
    });

    let _ = a.join();
    let _ = b.join();
}
