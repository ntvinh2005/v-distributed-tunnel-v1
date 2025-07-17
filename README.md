# V-Distributed Tunnel (QUIC) – Comprehensive Guide

This document thoroughly guides through setting up, configuring, and testing your own Rust-based secure QUIC tunnel, including managing tunnel nodes using PostgreSQL and performing end-to-end testing using a local TCP echo server.

---

## Prerequisites

Ensure the following tools are installed:

- **Rust** (stable): [https://rustup.rs](https://rustup.rs)
- **OpenSSL** (for certificates):
  - Windows: [Win32/Win64 OpenSSL](https://slproweb.com/products/Win32OpenSSL.html)
  - macOS: `brew install openssl`
  - Linux: `sudo apt install openssl`

---

## 1. Generate Certificates

Create `server.conf`:

```ini
[ req ]
default_bits       = 4096
prompt             = no
default_md         = sha256
req_extensions     = req_ext
distinguished_name = dn

[ dn ]
CN = localhost

[ req_ext ]
subjectAltName = @alt_names

[ alt_names ]
DNS.1 = localhost

[ v3_ca ]
basicConstraints = critical,CA:FALSE
keyUsage = digitalSignature, keyEncipherment
extendedKeyUsage = serverAuth
subjectAltName = @alt_names
```

Generate certificates:

```sh
openssl req -x509 -newkey rsa:4096 -keyout key.pem -out cert.pem -days 365 -nodes -config server.conf -extensions v3_ca
```

This generates `key.pem` and `cert.pem`.

---

## 2. Build the Project

Build release version:

```sh
cargo build --release
```

Or debug version:

```sh
cargo build
```

---

## 3. Run Tunnel-Admin (Manage Nodes)

Start the node management CLI:

```sh
cargo run --bin tunnel-admin
```

Available commands:
- `list`: List nodes
- `add <node_id>`: Add node (requires node ID). This also generate a password randomly for you. Remember to use that password when authenticating when trying to connecting client node.
- `delete`: Remove node
- `view <node_id>`: View node details
- `help`: Display help
- `exit` or `quit`: Exit CLI

Example add node:

```sh
dugeon-master> add node1
```

---

## 4. Start the QUIC Server

```sh
cargo run --bin server
```

Server listens on UDP port 5000.

---

## 5. Start the QUIC Client

```sh
cargo run --bin client
```

Authenticate with node ID/password set earlier.

---

## 6. Setup Local Echo Server (Remote Tester)

This Go program creates a simple TCP echo server for tunnel testing.

Create `echo-server.go`:

```go
package main

import (
    "fmt"
    "io"
    "net"
)

func main() {
    listener, err := net.Listen("tcp", ":8080")
    if err != nil {
        panic(err)
    }
    fmt.Println("Echo server listening on :8080")

    for {
        conn, err := listener.Accept()
        if err != nil {
            fmt.Println("Accept error:", err)
            continue
        }
        go func(c net.Conn) {
            defer c.Close()
            io.Copy(c, c)
        }(conn)
    }
}
```

Run echo server:

```sh
go run echo-server.go
```

---

## 7. Test the Tunnel End-to-End

Use netcat (`nc`) or this simple Go tester script:

`tcp-tester.go`:

```go
package main

import (
    "bufio"
    "fmt"
    "net"
    "os"
)

func main() {
    conn, err := net.Dial("tcp", "localhost:<assigned_port>")
    if err != nil {
        fmt.Println("Connection error:", err)
        return
    }
    defer conn.Close()

    fmt.Println("Connected to tunnel! Type messages:")
    go func() {
        io.Copy(os.Stdout, conn)
    }()

    scanner := bufio.NewScanner(os.Stdin)
    for scanner.Scan() {
        line := scanner.Text()
        fmt.Fprintln(conn, line)
    }
}
```

Replace `<assigned_port>` with port from client terminal.

Run the tester:

```sh
go run tcp-tester.go
```

Any message typed will be echoed back through the tunnel!

---

## 8. Troubleshooting

- Ensure `cert.pem` matches on both server/client.
- Check PostgreSQL connection string correctness.
- Confirm port availability (5000 for QUIC, 8080 for Echo).

---

## Contributing

Pull requests, bug reports, and feature requests are welcome! This project is ideal for exploring Rust async, QUIC protocols, and secure tunneling.
Or please just give me a star if you are reading it (You must feel interested enough to read until this point)

---

## License

[MIT License](LICENSE)
