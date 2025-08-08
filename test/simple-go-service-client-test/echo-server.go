package main

import (
	"fmt"
	"net/http"
)

func main() {
	http.HandleFunc("/", func(w http.ResponseWriter, r *http.Request) {
		fmt.Println("Received request for", r.URL.Path)
		fmt.Fprintf(w, "Hello from tunnel! You requested: %s\n", r.URL.Path)
	})

	fmt.Println("HTTP server listening on :8080")
	err := http.ListenAndServe(":8080", nil)
	if err != nil {
		panic(err)
	}
}
