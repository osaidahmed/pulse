package main

import (
	"fmt"
	"os"
	"runtime"
)

var debugMode bool
var logLevel string
var maxWorkers int

func init() {
	if os.Getenv("DEBUG") == "true" {
		debugMode = true
		fmt.Println("Debug mode enabled")
	}

	if os.Getenv("LOG_LEVEL") != "" {
		logLevel = os.Getenv("LOG_LEVEL")
	} else {
		logLevel = "info"
	}

	if runtime.NumCPU() > 4 {
		maxWorkers = runtime.NumCPU()
	} else {
		maxWorkers = 4
	}

	if os.Getenv("ENV") == "production" {
		if debugMode {
			fmt.Println("Warning: debug mode in production")
		}
	}
}

func GetConfig() (bool, string, int) {
	return debugMode, logLevel, maxWorkers
}
