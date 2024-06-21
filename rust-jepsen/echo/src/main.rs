use std::io::{Stdout, Write};
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
struct InitRequestMessage {
    src: String,
    dest: String,
    body: InitRequestBody,
}

#[derive(Default, Debug, Clone, Serialize, Deserialize)]
struct InitRequestBody {
    // can't use 'type' as the field name since it's a Rust keyword
    #[serde(rename = "type")]
    msg_type: String,
    msg_id: u32,
    node_id: String,
    node_ids: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
struct InitResponseMessage {
    src: String,
    dest: String,
    body: InitResponseBody,
}

#[derive(Default, Debug, Clone, Serialize, Deserialize)]
struct InitResponseBody {
    // can't use 'type' as the field name since it's a Rust keyword
    #[serde(rename = "type")]
    msg_type: String,
    in_reply_to: u32,
}

#[derive(Default, Debug, Clone, Serialize, Deserialize)]
struct EchoMessage {
    src: String,
    dest: String,
    body: EchoBody,
}

#[derive(Default, Debug, Clone, Serialize, Deserialize)]
struct EchoBody {
    // can't use 'type' as the field name since it's a Rust keyword
    #[serde(rename = "type")]
    msg_type: String,
    msg_id: u32,
    #[serde(skip_deserializing)]
    in_reply_to: u32,
    echo: String,
}

fn main() {
    let stdout = &std::io::stdout();
    let stdin = &mut std::io::stdin();
    let mut reader = serde_json::Deserializer::from_reader(stdin);

    let (reply_to, node_name, msg_id) =
        if let Ok(init) = InitRequestMessage::deserialize(&mut reader)
            .map(|t| dbg!(t))
            .map_err(|e| dbg!(e)) {
            assert_eq!("init", init.body.msg_type);
            (init.src, init.body.node_id, init.body.msg_id)
        } else {
            // failed deserializing init request
            panic!("invalid init message");
        };

    let init_resp = InitResponseMessage {
        src: node_name.clone(),
        dest: reply_to,
        body: InitResponseBody {
            msg_type: String::from("init_ok"),
            in_reply_to: msg_id,
        },
    };

    write_serializable(stdout, dbg!(init_resp));

    let mut id = 1u32;

    loop {
        if let Ok(message) = EchoMessage::deserialize(&mut reader).map_err(|e| dbg!(e)) {
            let repl = EchoMessage {
                src: node_name.clone(),
                dest: message.src.clone(),
                body: EchoBody {
                    msg_type: String::from("echo_reply"),
                    msg_id: id,
                    in_reply_to: message.body.msg_id,
                    echo: message.body.echo.clone(),
                },
            };

            write_serializable(stdout, repl);

            id += 1;
        }
    }
}

fn write_serializable(mut stdout: &Stdout, mesg: impl Serialize) {
    let res = serde_json::to_writer(stdout, &mesg);
    let _ = stdout.flush();
    if let Some(write_error) = res.err() {
        eprintln!("{write_error}");
    }
}
