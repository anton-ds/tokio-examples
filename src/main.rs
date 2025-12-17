use tokio::net::{TcpListener, TcpStream};
use tokio::io::{AsyncReadExt, AsyncWriteExt};
use std::sync::Arc;
use std::sync::Mutex;

// Test struct, used only to demonstrate move semantics
#[derive(Debug)]
struct Test {
    test: i32,
}

struct State {
    counter: Mutex<i32>,
}

impl State {
    fn new() -> Self {
        Self {
            counter: Mutex::new(0),
        }
    }

    fn increment(&self) -> i32 {
        // Lock is acquired and released inside a synchronous method
        // to guarantee it is never held across an `.await`
        let mut lock = self.counter.lock().unwrap();
        *lock += 1;
        *lock
    } // mutex is free
}

// The current_thread runtime flavor is a lightweight, single-threaded runtime.
// It is a good choice when only spawning a few tasks and opening a handful of sockets.
// For example, this option works well when providing a synchronous API bridge
// on top of an asynchronous client library.
// #[tokio::main(flavor = "current_thread")]

#[tokio::main]
async fn main() {
    // Shared state for all connections
    let state = Arc::new(State::new());

    // TCP server
    let listener = TcpListener::bind("127.0.0.1:7000")
        .await
        .unwrap();

    println!("Server listening on 127.0.0.1:7000");

    loop {
        // Wait for an incoming connection
        let (socket, _) = listener.accept().await.unwrap();
        // Arc cloning is cheap; it only increments the reference counter
        let state = state.clone();
        // Used only to demonstrate ownership transfer into the spawned task
        let test = Test{ test: 1 };

        // Each connection is handled in a separate task
        // Variables used inside the spawned task are moved into it
        tokio::spawn(async move {
            println!("Using test value: {:?}", test.test);
            handle_tcp_request(socket, state).await;
        });

        // `test` is no longer accessible here because it was moved
        // test;
    }
}

async fn handle_tcp_request(
    mut socket: TcpStream,
    state: Arc<State>,
) {
    let mut buf = [0u8; 1024];

    loop {
        let n = socket.read(&mut buf).await.unwrap();

        // Client closed the connection
        if n == 0 {
            break;
        }

        // `from_utf8_lossy` is used to tolerate invalid UTF-8 input
        let input = String::from_utf8_lossy(&buf[..n]).trim().to_string();

        let current = state.increment();

        let response = format!(
            "OK: '{}' (request #{})\n",
            input, current,
        );

        socket.write_all(response.as_bytes()).await.unwrap();

        println!("Received: {} (request #{})", input, current);
    }
}
