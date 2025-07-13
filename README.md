# V-Distributed Tunnel (QUIC) – Quick Start

This document's purpose is to guild others in building, configuring, and testing their Rust-based QUIC tunnel—connecting a server and client securely on their local machine.

---

## Prerequisites

- Rust (stable): https://rustup.rs/
- OpenSSL (for generating certificates):
  - On Windows: Win32/Win64 OpenSSL: https://slproweb.com/products/Win32OpenSSL.html
  - On macOS: `brew install openssl`
  - On Linux: `sudo apt install openssl`

---

## 1. Generate Self-Signed Certificates

You need a certificate and key (not a CA). Create a config file called `server.conf` with this content:

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

Then run:

```sh
openssl req -x509 -newkey rsa:4096 -keyout key.pem -out cert.pem -days 365 -nodes -config server.conf -extensions v3_ca
```

This produces `key.pem` and `cert.pem`.

---

## 2. Build the Project

```sh
cargo build --release
```

Or for debug mode:

```sh
cargo build
```

---

## 3. Start the Server

In one terminal:

```sh
cargo run --bin server
```

The server will listen on UDP port 5000.

---

## 4. Start the Client

In a second terminal:

```sh
cargo run --bin client
```

The client will connect to `localhost:5000` (edit the code if you want to use a remote server).

---

## 5. Confirm Success

- The server terminal should show new connection and received data.
- The client should print something like:

  ```
  Connected to 127.0.0.1:5000
  Received: [some ASCII bytes, find out the secret yourself]
  ```

If you see an error about `invalid peer certificate: ...CaUsedAsEndEntity`, your certificate was not generated with `CA:FALSE`—regenerate using the config above.

---

## 6. Troubleshooting

- Make sure both binaries use the same `cert.pem`.
- If running across different machines, copy `cert.pem` to both and set the client’s server IP.

---

## Contributing

This project is a great place to start learning async Rust, QUIC, and secure tunnels.
Pull requests and improvements are welcome.

---

## License

[![License: MIT](https://img.shields.io/badge/License-MIT-yellow.svg)](LICENSE)