package main

import (
	"bufio"
	"fmt"
	"io"
	"net"
	"os"
)

func main() {
	if len(os.Args) != 2 {
		fmt.Println("Usage: go run remote-tester.go <port>")
		return
	}
	port := os.Args[1]
	server := "127.0.0.1:" + port
	fmt.Printf("Connecting to %s...\n", server)

	conn, err := net.Dial("tcp", server)
	if err != nil {
		fmt.Println("Connection error:", err)
		return
	}
	defer conn.Close()

	fmt.Println("Connected! Type and press Enter. Ctrl+C to exit.")

	go func() {
		io.Copy(os.Stdout, conn)
	}()

	scanner := bufio.NewScanner(os.Stdin)
	for scanner.Scan() {
		line := scanner.Text()
		if _, err := fmt.Fprintln(conn, line); err != nil {
			fmt.Println("Send error:", err)
			return
		}
	}
}
