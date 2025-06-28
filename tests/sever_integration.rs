use std::{
    io::{Read, Write},
    net::{SocketAddr, TcpListener},
    process::{Child, Command},
    thread,
    time::Duration,
};

fn start_server() -> (Child, SocketAddr) {
    let listener = TcpListener::bind("127.0.0.1:0").unwrap();
    let addr = listener.local_addr().unwrap();
    drop(listener);
    let child = Command::new(env!("CARGO_BIN_EXE_dllbridge32"))
        .arg("testlib/lib_linux.so")
        .arg(addr.port().to_string())
        .spawn()
        .unwrap();

    thread::sleep(Duration::from_millis(50));
    (child, addr)
}

#[test]
fn smoke_test() {
    let (mut child, _addr) = start_server();
    child.kill().ok();
}

#[test]
fn hello_world() {
    let (mut child, addr) = start_server();

    let mut stream =
        std::net::TcpStream::connect(("127.0.0.1", addr.port())).expect("Couldn't start listener!");

    println!("Writing to stream");
    stream
        .write_all(b"call helloworld sig:void ->int\n")
        .expect("Couldnt not write to stream!");

    let mut buf: Vec<u8> = Vec::new();
    stream
        .read_to_end(&mut buf)
        .expect("Couldn`t read from stream");
    println!("Buffer: {:#?}", buf);

    child.kill().ok();
}
