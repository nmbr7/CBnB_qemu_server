use serde::{Deserialize, Serialize};
use serde_json::Result;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum ServiceType {
    // Send to the node
    Storage,
    Faas,
    Paas,
    // CUSTOM,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum ServiceMsgType {
    // CHECKSYSTAT,
    SERVICEUPDATE,
    SERVICEINIT,
    SERVICESTART,
    SERVICESTOP,
    // CUSTOM,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ServiceMessage {
    pub msg_type: ServiceMsgType,
    pub service_type: ServiceType,
    pub content: String,
    pub uuid: String,
}

//////////////////////////////////////////////////////////

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum Message {
    Service(ServiceMessage),
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn run() {
        //  println!("{:?}",Message::new(MsgType::REGISTER,stat))
        //println!("{}", Message::<sys_stat::NodeResources>::register(stat))
    }
}
