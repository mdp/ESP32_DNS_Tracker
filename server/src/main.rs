extern crate clap;

use std::{net::{UdpSocket}, time::SystemTime};
use std::net::{Ipv4Addr};

use chrono::{DateTime, Utc};

use clap::{App};

mod dns;
mod message_handler;

use dns::{BytePacketBuffer, DnsPacket, DnsRecord, ResultCode};
use message_handler::{MessageBufferCache, MessageChunk};

type Error = Box<dyn std::error::Error>;
type Result<T> = std::result::Result<T, Error>;

#[derive(Debug)]
struct MessageResult {
    id: String,
    is_complete: bool
}

fn get_unix_epoch_bytes() -> [u8; 8] {
    SystemTime::now().duration_since(SystemTime::UNIX_EPOCH).expect("Can't get Unix time").as_secs().to_le_bytes()
}

fn get_checksum(val: &[u8]) -> u8 {
    let mut c: u32 = 0; 
    for i in val.iter() {
        c = c ^ *i as u32;
    }
    (c % 255) as u8
}

fn add_inbound_query(message_buffer_cache: &mut MessageBufferCache,  name: &str, domain: &str) -> Result<MessageResult> {
    println!("Received query: {:?}", name);
    let message_chunk = MessageChunk::from(name, domain)?;
    let id = message_chunk.id.clone();
    match message_buffer_cache.add(message_chunk) {
        Ok(is_complete) => {
            Ok(MessageResult{id, is_complete})
        },
        Err(e) => Err(e)
    }
}

fn handle_completed_message(msg: Vec<u8>, output_dir: &str, id: &str) -> Result<()>{
    // Turn into text
    // Parse JSON
    // Extract name of device and write to file
    //      datetime-id.json
    let now: DateTime<Utc> = Utc::now();
    let now_str = now.format("%Y-%m-%d_%H-%M-%S");
    let filename = format!("{}_{}", now_str, id);
    let filepath = std::path::Path::new(output_dir).join(filename);
    let filepath_str = filepath.to_str().unwrap();
    match std::str::from_utf8(&msg) {
        Ok(message_str) => {
            let destination = format!("{}.txt", filepath_str);
            std::fs::write(&destination, &message_str).unwrap_or_else(|_|
                panic!("Unable to write to: {}", &destination));
            println!("{:}", &message_str);
        }
        Err(_) => {
            let destination = format!("{}.bin", filepath_str);
            std::fs::write(&destination, &msg).unwrap_or_else(|_|
                panic!("Unable to write to: {}", &destination));
            println!("Wrote binary content to {}", filepath_str);
        }
    }

    return Ok(())
}


/// Handle a single incoming packet
fn handle_query(socket: &UdpSocket, message_buffer_cache: &mut MessageBufferCache, domain: &str) -> Result<MessageResult> {
    // With a socket ready, we can go ahead and read a packet. This will
    // block until one is received.
    let mut req_buffer = BytePacketBuffer::new();

    // The `recv_from` function will write the data into the provided buffer,
    // and return the length of the data read as well as the source address.
    // We're not interested in the length, but we need to keep track of the
    // source in order to send our reply later on.
    let (_, src) = socket.recv_from(&mut req_buffer.buf)?;

    // Next, `DnsPacket::from_buffer` is used to parse the raw bytes into
    // a `DnsPacket`.
    let mut request = DnsPacket::from_buffer(&mut req_buffer)?;


    // Create and initialize the response packet
    let mut packet = DnsPacket::new();
    packet.header.id = request.header.id;
    packet.header.recursion_desired = true;
    packet.header.recursion_available = true;
    packet.header.response = true;
    packet.header.authoritative_answer = false;

    let question = request.questions.pop();

    let mut result: Result<MessageResult> = Err(format!("Nothing in dns query found to process").into());
    if let Some(question) = question {

        result = add_inbound_query(message_buffer_cache, &question.name, domain);

        let time_bytes = get_unix_epoch_bytes();
        let time_checksum = get_checksum(&time_bytes[0..5]);

        // Since all is set up and as expected, the query can be forwarded to the
        // target server. There's always the possibility that the query will
        // fail, in which case the `SERVFAIL` response code is set to indicate
        // as much to the client. If rather everything goes as planned, the
        // question and response records as copied into our response packet.
        packet.questions.push(question.clone());
        packet.header.rescode = ResultCode::NOERROR;
        if result.is_ok() && result.as_ref().unwrap().is_complete {
            packet.answers.push(DnsRecord::A{
                domain: question.name.clone(),
                ttl: 255,
                addr: Ipv4Addr::new(11,time_bytes[3],time_bytes[4],time_checksum)
            });
        } else {
            packet.answers.push(DnsRecord::A{
                domain: question.name.clone(),
                ttl: 255,
                addr: Ipv4Addr::new(10, time_bytes[0],time_bytes[1],time_bytes[2])
            });
        }
    } // Being mindful of how unreliable input data from arbitrary senders can be, we
    // need make sure that a question is actually present. If not, we return `FORMERR`
    // to indicate that the sender made something wrong.
    else {
        packet.header.rescode = ResultCode::FORMERR;
    }

    // The only thing remaining is to encode our response and send it off!
    let mut res_buffer = BytePacketBuffer::new();
    packet.write(&mut res_buffer)?;

    let data = res_buffer.get_data()?;

    socket.send_to(data, src)?;
    result
}

fn main() -> Result<()> {

    let matches = App::new("dns_drop")
                          .version("1.0")
                          .author("Mark Percival <m@mdp.im>")
                          .about("Listen for location updates from IOT devices via DNS tunneling")
                          .args_from_usage(
                              "-p, --port=[PORT]        'Port to use, default 53'
                              -o, --out=[PORT]          'Output directory to save locations'
                              <DOMAIN>                  'Root domain'
                              -v...                     'Sets the level of verbosity'")
                          .get_matches();

    let port: u16 = matches.value_of("port").unwrap_or("53").parse().unwrap();
    let domain = matches.value_of("DOMAIN").unwrap();
    let output_dir: &str = matches.value_of("out").unwrap_or("");
    // Bind an UDP socket on port 2053
    let socket = UdpSocket::bind(("0.0.0.0", port))?;

    // For now, queries are handled sequentially, so an infinite loop for servicing
    // requests is initiated.
    let mut message_buffer_cache = MessageBufferCache::new(64);
    loop {
        match handle_query(&socket, &mut message_buffer_cache, domain) {
            Ok(message_result) => {
                if message_result.is_complete {
                    if let Some(message) = message_buffer_cache.get_value(&message_result.id) {
                        if let Err(e) = handle_completed_message(message, output_dir, &message_result.id) {
                            println!("Error {:?}", e);
                        }
                    }
                }
            }
            Err(e) => eprintln!("An error occurred: {}", e),
        }
    }
}
