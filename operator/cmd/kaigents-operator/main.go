package main

import (
	"fmt"
	"os"

	"github.com/jensjohansen/kaigents/operator/internal/version"
)

func main() {
	args := os.Args[1:]
	if len(args) > 0 && (args[0] == "version" || args[0] == "--version" || args[0] == "-v") {
		fmt.Println(version.Version)
		return
	}

	fmt.Printf("kaigents-operator %s\n", version.Version)
}
