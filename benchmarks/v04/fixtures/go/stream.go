package main

import (
	"fmt"
	"os"
	"strconv"
)

func main() {
	if len(os.Args) != 2 { os.Exit(2) }
	size, err := strconv.ParseInt(os.Args[1], 10, 64)
	if err != nil || size < 0 || size > 10000000 { os.Exit(2) }
	state := int64(1)
	var checksum int64
	for i := int64(0); i < size; i++ {
		state = (state * 48271) % 2147483647
		if state%7 == 0 { checksum = (checksum*31 + state) % 2147483647 }
	}
	fmt.Printf("checksum=%d processed=%d\n", checksum, size)
}
