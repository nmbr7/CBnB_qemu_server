use crate::message::{Message, ServiceMessage, ServiceMsgType, ServiceType};
use serde_json::{json, Value};
use std::collections::HashMap;
use std::fs::File;
use std::io::prelude::*;
use std::io::SeekFrom;
use std::net::TcpListener;
use std::net::TcpStream;
use std::process::Command;

// TODO make async
pub fn getfile(filename: String, addr: String, id: String, dest: &String) {
    let content = json!({
        "msg_type" :  "read",
        "filename" :  filename,
        "id"       :  id,
    })
    .to_string();

    let data = Message::Service(ServiceMessage {
        msg_type: ServiceMsgType::SERVICEINIT,
        service_type: ServiceType::Storage,
        content: content,
        uuid: id,
    });

    let msg_data = serde_json::to_string(&data).unwrap();
    //println!("{}",test["content"].as_str().unwrap(());

    let mut resp = [0; 2048];
    let mut destbuffer = [0 as u8; 2048];

    let mut stream = TcpStream::connect(addr).unwrap();
    //println!("{:?}", msg_data);
    stream.write_all(msg_data.as_bytes()).unwrap();
    stream.flush().unwrap();

    let no = stream.read(&mut resp).unwrap();
    let fsize: Value = serde_json::from_slice(&resp[0..no]).unwrap();
    let filesize = fsize["total_size"].as_u64().unwrap() as usize;

    let mut totalfilesize = 0 as usize;
    loop {
        let no = stream.read(&mut resp).unwrap();
        //println!("val {}",std::str::from_utf8(&resp[0..no]).unwrap());
        let metadata: Value = serde_json::from_slice(&resp[0..no]).unwrap();
        //println!("{}",metadata);
        if metadata["msg_type"].as_str().unwrap() == "End" {
            break;
        }

        let size = metadata["size"].as_u64().unwrap() as usize;
        let index = metadata["index"].as_u64().unwrap();
        let mut total = 0 as usize;
        let mut bufvec: Vec<u8> = vec![];
        stream.write_all(String::from("OK").as_bytes()).unwrap();
        stream.flush().unwrap();
        loop {
            // ERROR hangs when size is 13664 so fetch the total file size first and if   \
            //       the size is less than 65536 before reaching the end request for ret- \
            //       ransmission
            let mut dno = stream.read(&mut destbuffer).unwrap();
            if dno > size {
                dno = size;
            }
            total += dno;
            bufvec.append(&mut destbuffer[0..dno].to_vec());
            //println!("Total: {} - dno: {} - Size {}",total,dno,size);
            if total == size {
                break;
            }
        }

        {
            use std::fs::OpenOptions;
            let mut file = OpenOptions::new().write(true).open(dest.clone()).unwrap();
            //file.set_len(21312864).unwrap();
            let val = file.seek(SeekFrom::Start(index * 65536)).unwrap();
            //println!("seeked to offset {}",val);
            //let mut contents = vec![];
            //let mut handle = file.take(size)i;
            file.write_all(&bufvec.as_slice()).unwrap();
            file.flush().unwrap();
        }
        totalfilesize += total;
        if totalfilesize == filesize {
            break;
        }
    }
    println!(
        "File Download complete, Total File Size : {} bytes",
        totalfilesize
    );
}
