package main

import (
	"fmt"
	"os"
	"strconv"
)

func main() {
	if len(os.Args) != 2 { os.Exit(2) }
	rounds, err := strconv.ParseInt(os.Args[1], 10, 64)
	if err != nil || rounds < 0 || rounds > 4096 { os.Exit(2) }
	checksum := int64(0)
	for round := int64(0); round < rounds; round++ {
		values := make([]int64, 0)
		for index := int64(0); index < 32; index++ {
			values = append(values, (round*17+index*13)%1009)
		}
		for _, value := range values { checksum += value }
	}
	fmt.Printf("checksum=%d processed=%d\n", checksum, rounds*32)
}
