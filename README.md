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

## What happens

Any text sent by a TCP client is forwarded to a **dedicated logger task** via a Tokio `mpsc` channel and printed on the server side as `[LOG] ...`.

At the same time, the server runs an **independent background task** that asynchronously copies everything typed into the server's **standard input (STDIN)** into a file called `log.txt`.

This demonstrates that:
- TCP sockets, STDIN, and files are all treated as **asynchronous byte streams**
- the same I/O primitives (`AsyncRead`, `AsyncWrite`, `io::copy`) work uniformly across them
- multiple asynchronous tasks can perform I/O concurrently without blocking each other

The server responds to each TCP client with:
```
OK: '<input>' (request #N)
```
