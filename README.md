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

Any text sent by a client is forwarded to a logger task via a mpsc channel and printed on the server side as [LOG] ...
The server responds to the client with: `OK: '<input>' (request #N)`.
