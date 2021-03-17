use std::fs::File;
use std::io::{self, prelude::*};
use std::path::PathBuf;

use crate::common::ipc;

use crate::utils::consts::ZELLIJ_IPC_PIPE;
use daemonize::Daemonize;
use interprocess::local_socket::{LocalSocketListener, LocalSocketStream};

pub fn start_server() {
    let listener = match LocalSocketListener::bind(ZELLIJ_IPC_PIPE) {
        Ok(sock) => sock,
        Err(err) => match err.kind() {
            io::ErrorKind::NotFound => {
                let mut dir_path = PathBuf::from(ZELLIJ_IPC_PIPE);
                dir_path.pop();
                std::fs::create_dir_all(dir_path).unwrap();
                LocalSocketListener::bind(ZELLIJ_IPC_PIPE).unwrap()
            }
            io::ErrorKind::AddrInUse => {
                std::fs::remove_file(ZELLIJ_IPC_PIPE).unwrap();
                LocalSocketListener::bind(ZELLIJ_IPC_PIPE).unwrap()
            }
            _ => panic!("{:?}", err),
        },
    };

    match Daemonize::new()
        .exit_action(|| println!("server running"))
        .start()
    {
        Ok(_) => event_loop(listener),
        Err(err) => {
            panic!("{:?}", err)
        }
    }
}

fn handle_error(conn: io::Result<LocalSocketStream>) -> Option<LocalSocketStream> {
    match conn {
        Ok(val) => Some(val),
        Err(error) => {
            eprintln!("Incoming connection failed: {}", error);
            None
        }
    }
}

fn event_loop(listener: LocalSocketListener) {
    for mut conn in listener.incoming().filter_map(handle_error) {
        let mut file = File::create("/tmp/server_log.txt").unwrap();
        file.write_all(b"In event loop").unwrap();
        let incoming_msg: ipc::ClientToServerMsg = serde_json::from_reader(&mut conn).unwrap();
        file.write_all(format!("Received message: {:?}", incoming_msg).as_bytes())
            .unwrap();
        match incoming_msg {
            ipc::ClientToServerMsg::CreateSession => {
                let new_session = ipc::Session {
                    id: 12345,
                    conn_name: String::from("bar"),
                    alias: String::from("foo"),
                };
                let outgoing_msg = ipc::ServerToClientMsg::SessionInfo(new_session);
                serde_json::to_writer(&mut conn, &outgoing_msg);
            }
            _ => {
                // Don't do anything
            }
        }
    }
}
