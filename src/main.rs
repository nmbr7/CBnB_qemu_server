mod api;
mod message;

use serde_json::{json, Value};
use tokio::io;
use tokio::io::{AsyncReadExt, AsyncWriteExt};
use tokio::net::{TcpListener, TcpStream};

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
    let map: HashMap<String, String> = HashMap::new();
    let ip_map = Arc::new(Mutex::new(map));

    let mut listener = TcpListener::bind(listen_addr).await?;

    while let Ok((inbound, _)) = listener.accept().await {
        let ip_map_copy = Arc::clone(&ip_map);
        let data = (inbound.peer_addr().unwrap().to_string());
println!("{}",data);
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
    ip_map: Arc<Mutex<HashMap<String, String>>>,
) -> Result<(), Box<dyn Error>> {
    // TODO Parse the msg for and run corresponding operations
    let mut resp = [0; 2048];
    let no = inbound.read(&mut resp).await?;
    println!("{}", std::str::from_utf8(&resp[0..no]).unwrap());
    let message: Value = serde_json::from_slice(&resp[0..no]).unwrap();
    println!("{:?}", message["msg_type"]);

    match message["msg_type"].as_str().unwrap() {
        "deploy" => {
            let file_id = message["fileid"].as_str().unwrap().to_string();
            let file_name = message["filename"].as_str().unwrap().to_string();
            let addr = format!("10.0.2.2:7779");

            use std::path::PathBuf;
            let mut app_root_path = PathBuf::new();

            app_root_path.push(r"/tmp/app_root");
            let app_root = app_root_path.to_str().unwrap().to_string();

            match message["runtime"].as_str().unwrap() {
                "python" => {
                    app_root_path.push("python");
                }
                "rust" => {
                    app_root_path.push("rust");
                }
                _ => {}
            }

            // TODO Generate a random filename
            app_root_path.set_file_name(file_name.clone());
            let appzip = app_root_path.to_str().unwrap().to_string();
            println!("Downloading the app files");
            getfile(file_name, addr, file_id, &appzip);
            app_cmd(&app_root, vec!["unzip", &appzip]).await?;
            println!("Unzipped the app files");
            // TODO Parse the app.yaml file to get the Details of the app
            // Based on the details got, create a Dockerfile depending on the language

            app_root_path.set_file_name("appzipname");
            let app_root_dir = app_root_path.to_str().unwrap().to_string();
            app_cmd(&app_root_dir, vec!["build", "tag_name"]).await?;
            app_cmd(&app_root_dir, vec!["start", "name", "tag_name"]).await?;

            let ips = app_cmd(&app_root, vec!["getip"]).await?;
            let cont_ip = ips.split(" - ").collect::<Vec<&str>>()[1].to_string();
            {
                let mut ip_map_mut = ip_map.lock().unwrap();
                ip_map_mut.insert("name".to_string(), cont_ip);
            }
        }
        "invoke" => {
            inbound.write("OK".as_bytes()).await?;
            let mut server_addr = String::new();
            {
                let ip_map_mut = ip_map.lock().unwrap();
                server_addr = ip_map_mut.get(&"name".to_string()).unwrap().to_string();
            }
            println!("Proxying to: {}", server_addr);
            let d_conn = docker_conn(inbound, server_addr.clone()).await?;
        }
        _ => {}
    }
    Ok(())
}

async fn app_cmd(app_root: &String, cmd: Vec<&str>) -> Result<String, Box<dyn Error>> {
    use std::process::{Command, Stdio};

    let cmdstr = match cmd[0] {
        "build" => format!("docker build -t {} .", cmd[1]),
        "start" => format!("docker run --detach --name {} --rm -t {}", cmd[1], cmd[2]),
        "stop" => format!("docker stop {}", cmd[1]),
        "getallip" => format!(
            "docker inspect -f '{{.Name}} - {{.NetworkSettings.IPAddress }}' $(docker ps -aq)"
        ),
        "getip" => format!(
            "docker inspect -f '{{.Name}} - {{.NetworkSettings.IPAddress }}' {}",
            cmd[1]
        ),
        "unzip" => format!("unzip {} -d {}", cmd[1],cmd[1].trim_end_matches(".zip")),
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
