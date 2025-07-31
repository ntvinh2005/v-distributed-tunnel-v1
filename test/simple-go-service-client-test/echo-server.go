// package main

// import (
// 	"fmt"
// 	"net"
// 	"os"
// 	"time"
// )

//	func main() {
//		conn, err := net.Dial("tcp", "127.0.0.1:YOUR_PUBLIC_PORT")
//		if err != nil {
//			fmt.Println("Dial error:", err)
//			os.Exit(1)
//		}
//		defer conn.Close()
//		buf := make([]byte, 65536)
//		start := time.Now()
//		sent := int64(0)
//		limit := int64(500 * 1024 * 1024) //Limit at 500MB
//		for sent < limit {
//			n, err := conn.Write(buf)
//			if err != nil {
//				fmt.Println("Write error:", err)
//				break
//			}
//			sent += int64(n)
//		}
//		elapsed := time.Since(start).Seconds()
//		mb := float64(sent) / 1024 / 1024
//		fmt.Printf("Sent %.2f MB in %.2f s (%.2f MB/s)\n", mb, elapsed, mb/elapsed)
//	}
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
