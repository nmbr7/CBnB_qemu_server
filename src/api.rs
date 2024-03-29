use crate::message::{Message, ServiceMessage, ServiceMsgType, ServiceType};
use indicatif::{ProgressBar, ProgressStyle};
use serde_json::{json, Value};
use std::collections::HashMap;
use std::fs::File;
use std::io::prelude::*;
use std::io::SeekFrom;
use std::net::TcpListener;
use std::net::TcpStream;
use std::process::Command;

pub fn initproxy(stream: &mut TcpStream, addr: String) {
    stream.write_all(addr.as_bytes()).unwrap();
    stream.flush().unwrap();
    let mut resp = [0; 2048];
    let no = stream.read(&mut resp).unwrap();
}

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
    let test = false;
    let mut stream = if test {
        let addr = format!("10.0.2.2:9090");
        TcpStream::connect(&addr).unwrap()
    } else {
        TcpStream::connect(&addr).unwrap()
    };
    if test{
        let addr = format!("172.28.5.77:7779");
        initproxy(&mut stream, addr.clone());
    }
    /*
    let connector = TlsConnector::new().unwrap();
    let stream = TcpStream::connect(&addr).unwrap();
    let mut stream = connector.connect(&addr.split(":").collect::<Vec<&str>>()[0], stream).unwrap();
    */

    //println!("{:?}", msg_data);
    stream.write_all(msg_data.as_bytes()).unwrap();
    stream.flush().unwrap();

    let no = stream.read(&mut resp).unwrap();
    println!("{}", std::str::from_utf8(&resp[0..no]).unwrap());
    let fsize: Value = serde_json::from_slice(&resp[0..no]).unwrap();
    let filesize = fsize["total_size"].as_u64().unwrap() as usize;

    stream.write_all(String::from("OK").as_bytes()).unwrap();
    stream.flush().unwrap();
    let mut totalfilesize = 0 as usize;

    let pb = ProgressBar::new(filesize as u64);
    pb.set_style(ProgressStyle::default_bar()
        .template("{spinner:.green} [{elapsed_precise}] [{bar:40.cyan/blue}] {bytes}/{total_bytes} ({eta})")
        .progress_chars("#>-"));
    loop {
        let no = stream.read(&mut resp).unwrap();
        stream.write_all(String::from("OK").as_bytes()).unwrap();
        stream.flush().unwrap();
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
        let mut destbuffer: Vec<u8> = vec![];
        loop {
            // ERROR hangs when size is 13664 so fetch the total file size first and if   \
            //       the size is less than 65536 before reaching the end request for ret- \
            //       ransmission
            //let mut dno = stream.read_to_end(&mut destbuffer).unwrap();
            for byte in stream.try_clone().unwrap().bytes() {
                total +=1;
                destbuffer.push(byte.unwrap());
                if total >= size{
                    break;
                }
             }
            /*if dno > size {
                dno = size;
            }*/
            bufvec.append(&mut destbuffer[0..total].to_vec());
            
            //println!("First 10 Bytes {:?}",bufvec[0..10].to_vec());
            //println!("Last  10 Bytes {:?}",bufvec[total-10..total].to_vec());
            //println!("Total: {} - Size {}",total,size);
            if total >= size {
                //println!("Total: {} - dno: {} - Size {}",total,dno,size);
                stream.write_all(String::from("OK").as_bytes()).unwrap();
                stream.flush().unwrap();
                break;
            }
        }

        totalfilesize += total;
        pb.set_position(totalfilesize as u64);
        {
            use std::fs::OpenOptions;
            let mut file = OpenOptions::new()
                .create(true)
                .write(true)
                .open(dest.clone())
                .unwrap();
            //file.set_len(21312864).unwrap();
            let val = file.seek(SeekFrom::Start(index * 4046848)).unwrap();
            //println!("seeked to offset {}",val);
            //let mut contents = vec![];
            //let mut handle = file.take(size)i;
            file.write_all(&bufvec.as_slice()).unwrap();
            file.flush().unwrap();
        }


        //println!("totalfilesize fetch so far: {}",totalfilesize);
        if totalfilesize == filesize {
            break;
        }
    }

    pb.finish_with_message("downloaded");
    println!(
        "File Download complete, Total File Size : {} bytes",
        totalfilesize
    );
}
