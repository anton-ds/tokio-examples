use std::future::Future;
use std::pin::Pin;
use std::sync::Arc;
use std::sync::Mutex;
use std::task::{Context, Poll};
use tokio::fs::File;
use tokio::io::{AsyncReadExt, AsyncWriteExt};
use tokio::io;
use tokio::net::{TcpListener, TcpStream};
use tokio::sync::mpsc;

/// Message sent to the logging task.
/// Each message represents a line received from a client.
#[derive(Debug)]
struct LogMessage {
    text: String,
}

/// Test struct, used only to demonstrate move semantics
#[derive(Debug)]
struct Test {
    test: i32,
}

/// Current state for transferring between threads
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

/// WaitForStateMachine is a custom Future that completes
/// when the shared request counter reaches a terminal state.
///
/// This demonstrates a Future that:
/// - does NOT do work by itself
/// - observes real application state
/// - becomes ready when an external condition is met
struct WaitForStateMachine {
    state: Arc<State>,
    machine: CountState,
}

enum CountState {
    Start,
    Mid { note: String },
    Done,
}

impl WaitForStateMachine {
    fn new(state: Arc<State>) -> Self {
        Self {
            state,
            machine: CountState::Start,
        }
    }
}

impl Future for WaitForStateMachine {
    type Output = String;

    fn poll(
        self: Pin<&mut Self>,
        cx: &mut Context<'_>,
    ) -> Poll<Self::Output> {
        let this = self.get_mut();
        let current = {
            let lock = this.state.counter.lock().unwrap();
            *lock
        };

        match &mut this.machine {
            CountState::Start => {
                if current >= 3 {
                    this.machine = CountState::Mid {
                        note: "reached 3 requests".to_string(),
                    };
                }
                // ❗enqueue current task again, not for production!
                cx.waker().wake_by_ref();
                Poll::Pending
            }
            CountState::Mid { note } => {
                if current >= 5 {
                    let output = format!(
                        "Reached 5 total requests (note from mid-state: {})",
                        note
                    );
                    this.machine = CountState::Done;
                    Poll::Ready(output)
                } else {
                    // ❗enqueue current task again, not for production!
                    cx.waker().wake_by_ref();
                    Poll::Pending
                }
            }
            CountState::Done => Poll::Pending,
        }
    }
}

// The current_thread runtime flavor is a lightweight, single-threaded runtime.
// It is a good choice when only spawning a few tasks and opening a handful of sockets.
// For example, this option works well when providing a synchronous API bridge
// on top of an asynchronous client library.
// #[tokio::main(flavor = "current_thread")]

#[tokio::main]
async fn main() {
    // Channel used for logging client input.
    // mpsc = many producers (client handlers), single consumer (logger task)
    let (log_tx, mut log_rx) = mpsc::channel::<LogMessage>(100);

    // Dedicated task that owns the logging logic.
    // This task is the ONLY place where logging happens.
    tokio::spawn(async move {
        while let Some(msg) = log_rx.recv().await {
            println!("[LOG] {}", msg.text);
        }
    });

    // Background task demonstrating async I/O piping:
    // Everything typed into STDIN will be asynchronously written to log.txt.
    // This shows that stdin and files are just AsyncRead / AsyncWrite streams.
    tokio::spawn(async {
        let mut stdin = io::stdin();
        let mut file = File::create("log.txt").await.unwrap();

        if io::copy(&mut stdin, &mut file).await.is_err() {
            eprintln!("STDIN -> file copy failed");
        }
    });

    // Shared state for all connections
    let state = Arc::new(State::new());
    let wait_state = state.clone();

    // This background task demonstrates how a custom Future is used in practice.
    tokio::spawn(async move {
        let reached = WaitForStateMachine::new(wait_state).await;
        println!("{}", reached);
    });

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
        // For sending messages to the log channel
        let log_tx = log_tx.clone();
        // Used only to demonstrate ownership transfer into the spawned task
        let test = Test{ test: 1 };

        // Each connection is handled in a separate task
        // Variables used inside the spawned task are moved into it
        tokio::spawn(async move {
            println!("Using test value: {:?}", test.test);
            handle_tcp_request(socket, state, log_tx).await;
        });

        // `test` is no longer accessible here because it was moved
        // test;
    }
}

async fn handle_tcp_request(
    mut socket: TcpStream,
    state: Arc<State>,
    log_tx: mpsc::Sender<LogMessage>,
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

        // Instead of logging directly here, we send the message
        // to a dedicated logging task using message passing.
        // Send client input to the logger task via channel.
        // This decouples logging from request handling.
        let _ = log_tx.send(LogMessage {
            text: input.clone(),
        }).await;

        let current = state.increment();

        let response = format!(
            "OK: '{}' (request #{})\n",
            input, current,
        );

        socket.write_all(response.as_bytes()).await.unwrap();
    }
}