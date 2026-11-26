package main

import (
	"fmt"
	"os"
	"strconv"
)

func main() {
	if len(os.Args) != 10 { os.Exit(2) }
	size, err := strconv.ParseInt(os.Args[1], 10, 64)
	if err != nil || size < 0 || size > 5000000 { os.Exit(2) }
	var checksum int64
	for i := int64(0); i < size; i++ {
		value, parseErr := strconv.ParseInt(os.Args[(i%8)+2], 10, 64)
		if parseErr != nil { os.Exit(2) }
		checksum += value
	}
	fmt.Printf("checksum=%d processed=%d\n", checksum, size)
}
