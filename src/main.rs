mod api;
mod message;

use serde_json::{json, Value};
use tokio::io;
use tokio::io::{AsyncReadExt, AsyncWriteExt};
use tokio::net::{TcpListener, TcpStream};

use std::fs::File;
use std::fs::{self, DirBuilder};
use std::io::prelude::*;

use api::getfile;
use futures::future::try_join;
use futures::FutureExt;
use std::collections::HashMap;
use std::env;
use std::error::Error;
use std::sync::{Arc, Mutex};

#[tokio::main]
async fn main() -> Result<(), Box<dyn Error>> {
    // TODO lookp the docker ip from based of the app uuid
    let listen_addr = format!("0.0.0.0:8080");
    println!("Listening on: {}", listen_addr);
    let map: HashMap<String, Vec<String>> = HashMap::new();
    let ip_map = Arc::new(Mutex::new(map));

    let mut listener = TcpListener::bind(listen_addr).await?;

    while let Ok((inbound, _)) = listener.accept().await {
        let ip_map_copy = Arc::clone(&ip_map);
        let data = (inbound.peer_addr().unwrap().to_string());
        //println!("{}",data);
        let d_conn = server_handler(inbound, ip_map_copy).map(|r| {
            if let Err(e) = r {
                println!("Failed to init the thread ; error={}", e);
            }
        });
        tokio::spawn(d_conn);
    }
    Ok(())
}

async fn server_handler(
    mut inbound: TcpStream,
    ip_map: Arc<Mutex<HashMap<String, Vec<String>>>>,
) -> Result<(), Box<dyn Error>> {
    // TODO Parse the msg for and run corresponding operations
    let mut resp = [0; 2048];
    let no = inbound.read(&mut resp).await?;
    //println!("{}", std::str::from_utf8(&resp[0..no]).unwrap());
    let message: Value = serde_json::from_slice(&resp[0..no]).unwrap();
    //println!("{:?}", message["msg_type"]);

    match message["msg_type"].as_str().unwrap() {
        "deploy" => {
            let file_id = message["fileid"].as_str().unwrap().to_string();
            let file_name = message["filename"].as_str().unwrap().to_string();
            let tagname = message["tag"].as_str().unwrap().to_string();
            let addr = format!("10.0.2.2:7779");
            let name = tagname.clone();
            use std::path::PathBuf;
            let mut app_root_path = PathBuf::new();

            app_root_path.push(r"/tmp/app_root");

            match message["runtime"].as_str().unwrap() {
                "python" => {
                    app_root_path.push("python");
                }
                "rust" => {
                    app_root_path.push("rust");
                }
                _ => {}
            }
            let app_root = app_root_path.to_str().unwrap().to_string();
            //println!("{}",app_root);
            DirBuilder::new()
                .recursive(true)
                .create(&app_root_path)
                .unwrap();

            app_root_path.push("zipdir");

            app_root_path.set_file_name(file_name.clone());
            let appzip = app_root_path.to_str().unwrap().to_string();
            //println!("{}",appzip);
            println!("Downloading the app files");
            getfile(file_name, addr, file_id, &appzip);
            app_cmd(&app_root, vec!["unzip", &appzip]).await?;
            println!("Unzipped the app files");
            // TODO Parse the app.toml file to get the Details of the app
            // Based on the details got, create a Dockerfile depending on the language
            let mut app_vec = vec![String::from("None"),String::from("No Status Yet")];
            let app_root_dir = app_root_path
                .to_str()
                .unwrap()
                .trim_end_matches(".zip")
                .to_string();

            let dockerfile = format!(
                "FROM python:3\nWORKDIR /usr/src/app\nCOPY requirements.txt ./\nRUN pip install --no-cache-dir -r requirements.txt\nCOPY . .\nCMD [ \"python\", \"./main.py\" ]");
            //println!("{}",dockerfile);

            use std::fs::OpenOptions;
            //println!("{}",app_root_dir);

            let mut file = OpenOptions::new()
                .create(true)
                .write(true)
                .open(format!("{}/dockerfile", app_root_dir))
                .unwrap();

            file.write_all(dockerfile.as_bytes()).unwrap();
            file.flush().unwrap();
            println!("Building Docker image");
            app_vec[1] = format!("Building Docker image");
            {
                let mut ip_map_mut = ip_map.lock().unwrap();
                ip_map_mut.insert(tagname.to_string(), app_vec.clone());
            }
            app_cmd(&app_root_dir, vec!["build", &tagname]).await?;
            println!("starting the app");
            app_vec[1] = format!("Starting Docker container");
            {
                let mut ip_map_mut = ip_map.lock().unwrap();
                ip_map_mut.insert(tagname.to_string(), app_vec.clone());
            }
            app_cmd(&app_root_dir, vec!["start", &name, &tagname]).await?;

            let ips = app_cmd(&app_root, vec!["getip", &name]).await?;
            println!("At ip address {}", ips);
            app_vec[0] = format!("{}:8080", ips.trim().trim_matches('\''));
            app_vec[1] = format!("App started");
            {
                let mut ip_map_mut = ip_map.lock().unwrap();
                ip_map_mut.insert(tagname.to_string(), app_vec.clone());
            }
        }
        "invoke" => {
            println!("Got Conn");
            let name = message["tag"].as_str().unwrap().to_string();
            let mut server_addr = String::new();
            {
                let ip_map_mut = ip_map.lock().unwrap();
                server_addr = ip_map_mut.get(&name).unwrap_or(&vec!["None".to_string()])[0].to_string();
            }
            if server_addr == "None".to_string(){
                inbound.write("NO_APP".as_bytes()).await?;
                return Ok(());
            }
            else{
                inbound.write("OK".as_bytes()).await?;
            }
            println!("Proxying to: {}", server_addr);
            let d_conn = docker_conn(inbound, server_addr.clone()).await?;
        }
        "status" => {
            let name = message["tag"].as_str().unwrap().to_string();
            let mut status = String::new();
            {
                let ip_map_mut = ip_map.lock().unwrap();
                status = ip_map_mut.get(&name).unwrap_or(&vec!["None".to_string()])[1].to_string();
            }
            if status == "None".to_string(){
                inbound.write(status.as_bytes()).await?;
                return Ok(());
            }
            else{
                inbound.write("OK".as_bytes()).await?;
            }

            
            }
        _ => {}
    }
    Ok(())
}

async fn app_cmd(app_root: &String, cmd: Vec<&str>) -> Result<String, Box<dyn Error>> {
    use std::process::{Command, Stdio};

    let cmdstr = match cmd[0] {
        "build" => format!("docker build -t {} .", cmd[1]),
        "start" => format!("docker run --rm --read-only --detach --name {} --rm -t {}", cmd[1], cmd[2]),
        "stop" => format!("docker stop {}", cmd[1]),
        "getallip" => format!(
            "docker inspect -f '{{.Name}} - {{.NetworkSettings.IPAddress }}' $(docker ps -aq)"
        ),
        "getip" => format!(
            "docker inspect -f '{{{{.NetworkSettings.IPAddress}}}}' {}",
            cmd[1]
        ),
        "unzip" => format!("unzip {} -d {}", cmd[1], cmd[1].trim_end_matches(".zip")),
        _ => return Ok("Error".to_string()),
    };

    let args = cmdstr.split(" ").collect::<Vec<&str>>();
    let a = Command::new(&args[0])
        .args(&args[1..args.len()])
        .current_dir(app_root)
        .output()
        .expect("Error");

    // TODO Fetch the app files from cloud storage and unzip ( Use the download function from the cbnb cli )

    // TODO Create a container from the yaml or the corresponding config file

    Ok(std::str::from_utf8(&a.stdout).unwrap().to_string())
}

async fn docker_conn(mut inbound: TcpStream, proxy_addr: String) -> Result<(), Box<dyn Error>> {
    println!("Got a new connection");
    let mut outbound = TcpStream::connect(proxy_addr).await?;
    let (mut ri, mut wi) = inbound.split();
    let (mut ro, mut wo) = outbound.split();

    let client_to_server = io::copy(&mut ri, &mut wo);
    let server_to_client = io::copy(&mut ro, &mut wi);

    try_join(client_to_server, server_to_client).await?;

    Ok(())
}
