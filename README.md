# Tokio Examples

This is a small **educational Tokio example** showing how to:

- Run an asynchronous TCP server
- Handle each client connection in a separate Tokio task
- Share state safely using `Arc` + `std::sync::Mutex`
- Use **Tokio `mpsc` channels** for message passing
- Move logging logic into a **dedicated task**
- Avoid holding a mutex across `.await`

## How to run

```bash
cargo run
```

## How to connect

From another terminal:
```bash
telnet 127.0.0.1 7000
```
or
```bash
nc 127.0.0.1 7000
```

The server responds to each TCP client with:
```
OK: '<input>' (request #N)
```

## What happens

Any text sent by a TCP client is forwarded to a **dedicated logger task** via a Tokio `mpsc` channel and 
printed on the server side as `[LOG] ...`.

At the same time, the server runs an **independent background task** that asynchronously copies everything 
typed into the server's **standard input (STDIN)** into a file called `log.txt`.

In addition, the server includes a **custom Future example** used for educational purposes.

A background task awaits a Future that completes only when the total number of processed client requests reaches 
a predefined threshold. This demonstrates that:
- a `Future` does not perform work on its own
- a `Future` can observe real application state instead of time
- completion of a `Future` is driven by external events (client requests)
- tasks awaiting a `Future` are suspended without blocking CPU until the condition is met

Together, these background tasks show how Tokio treats different I/O sources
(TCP sockets, STDIN, files) in a uniform way.

Each of them is handled as an asynchronous byte stream, allowing the server to:
- read and write data without blocking the runtime
- run unrelated I/O operations in parallel tasks
- keep the core request-handling logic simple and focused
